use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;

use log::{debug, info};
use serde::Deserialize;

use crate::executor::{
    AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep, GgVersion,
};
use crate::fetch::fetch_json;
use crate::target::{Arch, Os, Target, Variant};

/// A tool published to PyPI, run from a venv on a gg-managed Python. The wheel
/// is the download, so versions are the package's own (`vibe@3` is vibe 3, not
/// python 3). The venv is built in post_prep_with_deps, once python is there.
pub struct PyPi {
    pub executor_cmd: ExecutorCmd,
    /// Tool name (cache dir name), e.g. "vibe"
    pub name: &'static str,
    /// PyPI project, e.g. "mistral-vibe"
    pub package: &'static str,
    /// Console script the package installs, e.g. "vibe"
    pub bin: &'static str,
}

/// CPython minor the venv is built on. Not "latest": a tool's dependencies
/// trail each new Python with their wheels, and 3.14 is what mistral-vibe's
/// own installer asks for.
const PYTHON_MINOR: u64 = 14;

#[derive(Deserialize)]
struct PyPiProject {
    releases: HashMap<String, Vec<PyPiFile>>,
}

#[derive(Deserialize)]
struct PyPiFile {
    filename: String,
    url: String,
    packagetype: String,
    #[serde(default)]
    yanked: bool,
}

/// `{dist}-{version}[-{build}]-{python}-{abi}-{platform}.whl`
fn wheel_tags(filename: &str) -> Option<(&str, &str, &str)> {
    let stem = filename.strip_suffix(".whl")?;
    let mut parts = stem.rsplitn(4, '-');
    let platform = parts.next()?;
    let abi = parts.next()?;
    let python = parts.next()?;
    parts.next()?;
    Some((python, abi, platform))
}

/// "py313" -> 13, for both py3XY and cp3XY tags
fn tag_minor(tag: &str, prefix: &str) -> Option<u64> {
    tag.strip_prefix(prefix)?.parse().ok()
}

/// Does a wheel with these tags load on CPython 3.<minor>? "py3" is any 3.x
/// and "py313" is 3.13 and up, so pure wheels do; stable-ABI wheels built on
/// an older or equal CPython do; a wheel built against one exact CPython only
/// on that one. Tags can be compressed ("py2.py3"), so any one of them is enough.
fn runs_on_python(python: &str, abi: &str, minor: u64) -> bool {
    let exact = format!("cp3{minor}");
    python.split('.').any(|tag| match abi {
        "none" => tag == "py3" || tag_minor(tag, "py3").is_some_and(|v| v <= minor) || tag == exact,
        "abi3" => tag_minor(tag, "cp3").is_some_and(|v| v <= minor),
        _ => abi == exact && tag == exact,
    })
}

/// Platform tags are compressed too ("manylinux_2_17_x86_64.manylinux2014_x86_64")
fn runs_on_target(platform: &str, target: &Target) -> bool {
    let arch = match target.arch {
        Arch::X86_64 => "x86_64",
        Arch::Arm64 => "aarch64",
        Arch::Armv7 => "armv7l",
        Arch::Any => return false,
    };
    platform.split('.').any(|tag| {
        if tag == "any" {
            return true;
        }
        match target.os {
            Os::Linux => {
                let libc = match target.variant {
                    Some(Variant::Musl) => "musllinux",
                    _ => "manylinux",
                };
                tag.starts_with(libc) && tag.ends_with(&format!("_{arch}"))
            }
            Os::Mac => {
                let mac_arch = if arch == "aarch64" { "arm64" } else { arch };
                tag.starts_with("macosx_")
                    && (tag.ends_with(&format!("_{mac_arch}")) || tag.ends_with("_universal2"))
            }
            Os::Windows => {
                tag == if arch == "aarch64" {
                    "win_arm64"
                } else {
                    "win_amd64"
                }
            }
            Os::Any => false,
        }
    })
}

/// The oldest platform a tag runs on, so the least demanding wheel wins when a
/// release ships several: manylinux_2_17 over _2_28, macosx_10_9 over _11_0. gg
/// does not know the host's glibc or macOS version, so lowest is the safe pick.
fn platform_floor(platform: &str) -> (u64, u64) {
    platform
        .split('.')
        .map(|tag| {
            let mut parts = tag.split('_');
            match parts.next() {
                Some("manylinux1") => (2, 5),
                Some("manylinux2010") => (2, 12),
                Some("manylinux2014") => (2, 17),
                _ => {
                    let mut nums = parts.map_while(|p| p.parse::<u64>().ok());
                    (nums.next().unwrap_or(0), nums.next().unwrap_or(0))
                }
            }
        })
        .min()
        .unwrap_or((0, 0))
}

/// Plain X.Y.Z only. GgVersion reads "3.1.0rc1" as 3.1.0, and a release
/// candidate must not pass for the release.
fn is_final_version(version: &str) -> bool {
    version
        .split('.')
        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn select_downloads(project: PyPiProject, target: &Target) -> Vec<Download> {
    project
        .releases
        .into_iter()
        .filter(|(version, _)| is_final_version(version))
        .filter_map(|(version, files)| {
            let version = GgVersion::new(&version)?;
            let mut wheels: Vec<&PyPiFile> = files
                .iter()
                .filter(|f| f.packagetype == "bdist_wheel" && !f.yanked)
                .filter(|f| {
                    wheel_tags(&f.filename).is_some_and(|(python, abi, platform)| {
                        runs_on_python(python, abi, PYTHON_MINOR)
                            && runs_on_target(platform, target)
                    })
                })
                .collect();
            // A platform wheel over a pure one when a release has both, the
            // least demanding platform first, then by name so the pick is stable
            wheels.sort_by_key(|f| {
                let platform = wheel_tags(&f.filename).map(|t| t.2).unwrap_or("");
                let pure = platform.split('.').any(|tag| tag == "any");
                (pure, platform_floor(platform), f.filename.clone())
            });
            let wheel = wheels.first()?;
            Some(Download {
                download_url: wheel.url.clone(),
                version: Some(version),
                // Already matched to the target above
                os: Some(Os::Any),
                arch: Some(Arch::Any),
                variant: Some(Variant::Any),
                tags: HashSet::new(),
            })
        })
        .collect()
}

/// The downloaded .whl - a wheel is a zip, but by extension it is copied in as is
fn wheel_in(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "whl"))
}

/// The base interpreter can go away under a venv (python's cache dir removed),
/// and its scripts then die with a baffling "No such file or directory"
fn venv_is_sound(venv: &Path) -> bool {
    fs::read_to_string(venv.join("pyvenv.cfg")).is_ok_and(|cfg| {
        cfg.lines()
            .filter_map(|line| line.split_once('='))
            .any(|(key, home)| key.trim() == "home" && Path::new(home.trim()).is_dir())
    })
}

fn run(mut cmd: Command, what: &str) -> Result<(), String> {
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{what} failed: {s}")),
        Err(e) => Err(format!("{what} failed: {e}")),
    }
}

impl PyPi {
    fn venv_bin(&self, venv: &Path, bin: &str) -> PathBuf {
        if cfg!(windows) {
            venv.join("Scripts").join(format!("{bin}.exe"))
        } else {
            venv.join("bin").join(bin)
        }
    }
}

impl Executor for PyPi {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            let url = format!("https://pypi.org/pypi/{}/json", self.package);
            match fetch_json::<PyPiProject>(&url).await {
                Some(project) => select_downloads(project, &input.target),
                None => {
                    debug!("{}: could not read {url}", self.name);
                    vec![]
                }
            }
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(match input.target.os {
            Os::Windows => format!("venv/Scripts/{}.exe", self.bin),
            _ => format!("venv/bin/{}", self.bin),
        })]
    }

    // The venv comes after prep (it needs python, which preps alongside), so
    // a dir holding just the wheel is a finished download, not a stale one
    fn cached_install_is_valid(&self, app_path: &AppPath) -> bool {
        wheel_in(&app_path.install_dir).is_some()
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            vec![ExecutorDep::new(
                "python".to_string(),
                Some(format!("3.{PYTHON_MINOR}")),
            )]
        })
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        vec![".".to_string()]
    }

    fn post_prep_with_deps(&self, app_path: &AppPath, path_vars: &[String]) -> Result<(), String> {
        let dir = &app_path.install_dir;
        let venv = dir.join("venv");
        // pip writes the console script before it is done, so a killed install
        // leaves a runnable-looking venv - only the marker says it finished
        let done = venv.join("gg-installed");
        if done.is_file() && venv_is_sound(&venv) {
            return Ok(());
        }
        let wheel = wheel_in(dir).ok_or(format!("{}: no wheel in {}", self.name, dir.display()))?;

        // Only the prepped dirs, never a python off $PATH
        let paths = env::join_paths(path_vars).map_err(|e| e.to_string())?;
        let python = ["python3", "python"]
            .iter()
            .find_map(|p| which::which_in(p, Some(&paths), dir).ok())
            .ok_or(format!("{}: no python among the prepped tools", self.name))?;

        // Start over if an earlier attempt died halfway
        if venv.exists() {
            let _ = fs::remove_dir_all(&venv);
        }
        // venv is quiet for a few seconds before pip starts talking
        println!("{}: building a venv for {}", self.name, self.package);
        info!("{}: creating venv {:?} with {:?}", self.name, venv, python);
        let mut mkvenv = Command::new(&python);
        mkvenv.arg("-m").arg("venv").arg(&venv);
        run(mkvenv, "python -m venv")?;

        info!("{}: installing {:?}", self.name, wheel);
        let mut pip = Command::new(self.venv_bin(&venv, "python"));
        pip.arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--no-input")
            .arg(&wheel)
            // pip's download cache stays inside gg's cache dir, like npm's
            .env("PIP_CACHE_DIR", dir.join("pip_cache"))
            .env("PIP_DISABLE_PIP_VERSION_CHECK", "1");
        if let Err(e) = run(pip, "pip install") {
            let _ = fs::remove_dir_all(&venv);
            return Err(format!("{}: {e}", self.name));
        }
        fs::write(
            &done,
            wheel.file_name().unwrap_or_default().as_encoded_bytes(),
        )
        .map_err(|e| format!("{}: {e}", self.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(os: Os, arch: Arch, variant: Option<Variant>) -> Target {
        Target { os, arch, variant }
    }

    #[test]
    fn test_wheel_tags() {
        assert_eq!(
            wheel_tags("mistral_vibe-3.0.3-cp312-abi3-manylinux_2_28_x86_64.whl"),
            Some(("cp312", "abi3", "manylinux_2_28_x86_64"))
        );
        assert_eq!(
            wheel_tags("mistral_vibe-2.14.1-py3-none-any.whl"),
            Some(("py3", "none", "any"))
        );
        // Build number between version and tags
        assert_eq!(
            wheel_tags("foo-1.0-1-py2.py3-none-any.whl"),
            Some(("py2.py3", "none", "any"))
        );
        assert_eq!(wheel_tags("mistral_vibe-3.0.3.tar.gz"), None);
        assert_eq!(wheel_tags("broken.whl"), None);
    }

    #[test]
    fn test_runs_on_python() {
        assert!(runs_on_python("py3", "none", 14));
        assert!(runs_on_python("py2.py3", "none", 14));
        assert!(runs_on_python("py313", "none", 14));
        assert!(runs_on_python("py314", "none", 14));
        // "3.15 and up" is not us
        assert!(!runs_on_python("py315", "none", 14));
        assert!(runs_on_python("cp312", "abi3", 14));
        assert!(runs_on_python("cp314", "abi3", 14));
        assert!(runs_on_python("cp314", "cp314", 14));
        // Built against another exact CPython, or a newer stable ABI
        assert!(!runs_on_python("cp313", "cp313", 14));
        assert!(!runs_on_python("cp315", "abi3", 14));
        assert!(!runs_on_python("py2", "none", 14));
    }

    #[test]
    fn test_runs_on_target() {
        let linux = target(Os::Linux, Arch::X86_64, None);
        assert!(runs_on_target("manylinux_2_28_x86_64", &linux));
        assert!(runs_on_target(
            "manylinux_2_17_x86_64.manylinux2014_x86_64",
            &linux
        ));
        assert!(runs_on_target("any", &linux));
        assert!(!runs_on_target("manylinux_2_28_aarch64", &linux));
        assert!(!runs_on_target("musllinux_1_2_x86_64", &linux));
        assert!(!runs_on_target("win_amd64", &linux));

        let musl = target(Os::Linux, Arch::X86_64, Some(Variant::Musl));
        assert!(runs_on_target("musllinux_1_2_x86_64", &musl));
        assert!(!runs_on_target("manylinux_2_28_x86_64", &musl));

        let mac_arm = target(Os::Mac, Arch::Arm64, None);
        assert!(runs_on_target("macosx_11_0_arm64", &mac_arm));
        assert!(runs_on_target("macosx_10_9_universal2", &mac_arm));
        assert!(!runs_on_target("macosx_11_0_x86_64", &mac_arm));

        let mac_intel = target(Os::Mac, Arch::X86_64, None);
        assert!(runs_on_target("macosx_11_0_x86_64", &mac_intel));
        assert!(runs_on_target("macosx_10_9_universal2", &mac_intel));
        assert!(!runs_on_target("macosx_11_0_arm64", &mac_intel));

        let win = target(Os::Windows, Arch::X86_64, None);
        assert!(runs_on_target("win_amd64", &win));
        assert!(!runs_on_target("win_arm64", &win));
        assert!(!runs_on_target("win32", &win));
    }

    #[test]
    fn test_platform_floor() {
        assert_eq!(platform_floor("manylinux_2_28_x86_64"), (2, 28));
        assert_eq!(platform_floor("manylinux2014_x86_64"), (2, 17));
        assert_eq!(platform_floor("manylinux1_x86_64"), (2, 5));
        // Compressed: the loosest tag is what it needs
        assert_eq!(
            platform_floor("manylinux_2_17_x86_64.manylinux2014_x86_64.manylinux_2_28_x86_64"),
            (2, 17)
        );
        assert_eq!(platform_floor("musllinux_1_2_x86_64"), (1, 2));
        assert_eq!(platform_floor("macosx_11_0_arm64"), (11, 0));
        assert_eq!(platform_floor("win_amd64"), (0, 0));
        assert_eq!(platform_floor("any"), (0, 0));
    }

    #[test]
    fn test_select_prefers_the_least_demanding_platform_wheel() {
        let mut releases = HashMap::new();
        releases.insert(
            "1.0.0".to_string(),
            vec![
                file("tool-1.0.0-py3-none-any.whl", "bdist_wheel", false),
                file(
                    "tool-1.0.0-cp312-abi3-manylinux_2_28_x86_64.whl",
                    "bdist_wheel",
                    false,
                ),
                file(
                    "tool-1.0.0-cp312-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl",
                    "bdist_wheel",
                    false,
                ),
            ],
        );
        let downloads = select_downloads(
            PyPiProject { releases },
            &target(Os::Linux, Arch::X86_64, None),
        );
        assert_eq!(downloads.len(), 1);
        assert!(
            downloads[0]
                .download_url
                .ends_with("manylinux_2_17_x86_64.manylinux2014_x86_64.whl"),
            "{}",
            downloads[0].download_url
        );
    }

    fn file(filename: &str, packagetype: &str, yanked: bool) -> PyPiFile {
        PyPiFile {
            filename: filename.to_string(),
            url: format!("https://files.example/{filename}"),
            packagetype: packagetype.to_string(),
            yanked,
        }
    }

    fn project() -> PyPiProject {
        let mut releases = HashMap::new();
        releases.insert(
            "3.0.3".to_string(),
            vec![
                file(
                    "mistral_vibe-3.0.3-cp312-abi3-macosx_11_0_arm64.whl",
                    "bdist_wheel",
                    false,
                ),
                file(
                    "mistral_vibe-3.0.3-cp312-abi3-manylinux_2_28_x86_64.whl",
                    "bdist_wheel",
                    false,
                ),
                file(
                    "mistral_vibe-3.0.3-cp312-abi3-win_amd64.whl",
                    "bdist_wheel",
                    false,
                ),
                file("mistral_vibe-3.0.3.tar.gz", "sdist", false),
            ],
        );
        // Pure wheel era
        releases.insert(
            "2.14.1".to_string(),
            vec![
                file("mistral_vibe-2.14.1-py3-none-any.whl", "bdist_wheel", false),
                file("mistral_vibe-2.14.1.tar.gz", "sdist", false),
            ],
        );
        // Yanked wheel: not on offer
        releases.insert(
            "2.20.0".to_string(),
            vec![file(
                "mistral_vibe-2.20.0-py3-none-any.whl",
                "bdist_wheel",
                true,
            )],
        );
        // Source only: nothing to run
        releases.insert(
            "2.21.0".to_string(),
            vec![file("mistral_vibe-2.21.0.tar.gz", "sdist", false)],
        );
        // Pre-release
        releases.insert(
            "3.1.0rc1".to_string(),
            vec![file(
                "mistral_vibe-3.1.0rc1-py3-none-any.whl",
                "bdist_wheel",
                false,
            )],
        );
        // Built for one other CPython only
        releases.insert(
            "2.22.0".to_string(),
            vec![file(
                "mistral_vibe-2.22.0-cp313-cp313-manylinux_2_28_x86_64.whl",
                "bdist_wheel",
                false,
            )],
        );
        PyPiProject { releases }
    }

    fn versions(downloads: &[Download]) -> Vec<String> {
        let mut v: Vec<String> = downloads
            .iter()
            .filter_map(|d| d.version.as_ref().map(|v| v.to_string()))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn test_select_linux_picks_platform_wheel_per_final_version() {
        let downloads = select_downloads(project(), &target(Os::Linux, Arch::X86_64, None));
        assert_eq!(versions(&downloads), vec!["2.14.1", "3.0.3"]);
        let v3 = downloads
            .iter()
            .find(|d| d.version.as_ref().map(|v| v.to_string()).as_deref() == Some("3.0.3"))
            .unwrap();
        assert!(v3.download_url.ends_with("manylinux_2_28_x86_64.whl"));
    }

    #[test]
    fn test_select_mac_and_windows() {
        let mac = select_downloads(project(), &target(Os::Mac, Arch::Arm64, None));
        assert!(mac
            .iter()
            .any(|d| d.download_url.ends_with("macosx_11_0_arm64.whl")));
        let win = select_downloads(project(), &target(Os::Windows, Arch::X86_64, None));
        assert!(win
            .iter()
            .any(|d| d.download_url.ends_with("win_amd64.whl")));
        // Pure wheel fits everywhere
        assert!(versions(&win).contains(&"2.14.1".to_string()));
    }

    #[test]
    fn test_select_no_wheel_for_musl() {
        // No musllinux wheels published, so only the pure-python release fits
        let downloads = select_downloads(
            project(),
            &target(Os::Linux, Arch::X86_64, Some(Variant::Musl)),
        );
        assert_eq!(versions(&downloads), vec!["2.14.1"]);
    }

    fn test_pypi() -> PyPi {
        PyPi {
            executor_cmd: ExecutorCmd {
                cmd: "vibe".to_string(),
                version: None,
                distribution: None,
                include_tags: HashSet::new(),
                exclude_tags: HashSet::new(),
                gems: None,
            },
            name: "vibe",
            package: "mistral-vibe",
            bin: "vibe",
        }
    }

    #[test]
    fn test_wheel_in_marks_cache_valid() {
        let dir = tempfile::tempdir().unwrap();
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };
        assert!(!test_pypi().cached_install_is_valid(&app_path));
        fs::write(dir.path().join("gg-meta.json"), b"{}").unwrap();
        assert!(!test_pypi().cached_install_is_valid(&app_path));
        fs::write(
            dir.path()
                .join("mistral_vibe-3.0.3-cp312-abi3-manylinux_2_28_x86_64.whl"),
            b"PK",
        )
        .unwrap();
        assert!(test_pypi().cached_install_is_valid(&app_path));
    }

    fn fake_venv(dir: &Path, home: &Path, finished: bool) {
        let venv = dir.join("venv");
        let bin = test_pypi().venv_bin(&venv, "vibe");
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        fs::write(&bin, b"#!/bin/sh").unwrap();
        fs::write(
            venv.join("pyvenv.cfg"),
            format!("home = {}\nversion = 3.14.10\n", home.display()),
        )
        .unwrap();
        if finished {
            fs::write(
                venv.join("gg-installed"),
                b"mistral_vibe-3.0.3-py3-none-any.whl",
            )
            .unwrap();
        }
    }

    #[test]
    fn test_post_prep_with_deps_is_a_noop_once_the_venv_is_there() {
        let dir = tempfile::tempdir().unwrap();
        fake_venv(dir.path(), dir.path(), true);
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };
        // No python anywhere in path_vars, and it must not go looking
        assert_eq!(test_pypi().post_prep_with_deps(&app_path, &[]), Ok(()));
    }

    #[test]
    fn test_post_prep_with_deps_redoes_an_install_that_was_cut_short() {
        let dir = tempfile::tempdir().unwrap();
        // Script in place, marker missing: pip was killed mid-way
        fake_venv(dir.path(), dir.path(), false);
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };
        let err = test_pypi().post_prep_with_deps(&app_path, &[]).unwrap_err();
        assert!(err.contains("no wheel"), "{err}");
    }

    #[test]
    fn test_post_prep_with_deps_rebuilds_a_venv_whose_python_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        fake_venv(
            dir.path(),
            &dir.path().join("python-that-was-removed"),
            true,
        );
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };
        // Not a no-op any more: it wants to rebuild, and gets as far as the wheel
        let err = test_pypi().post_prep_with_deps(&app_path, &[]).unwrap_err();
        assert!(err.contains("no wheel"), "{err}");
    }

    #[test]
    fn test_post_prep_with_deps_needs_a_wheel_and_a_python() {
        let dir = tempfile::tempdir().unwrap();
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };
        let err = test_pypi().post_prep_with_deps(&app_path, &[]).unwrap_err();
        assert!(err.contains("no wheel"), "{err}");

        fs::write(
            dir.path().join("mistral_vibe-3.0.3-py3-none-any.whl"),
            b"PK",
        )
        .unwrap();
        let err = test_pypi().post_prep_with_deps(&app_path, &[]).unwrap_err();
        assert!(err.contains("no python"), "{err}");
    }
}
