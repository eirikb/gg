use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os, Target, Variant};

/// Python via astral's python-build-standalone (PBS) - the same prebuilt,
/// relocatable CPython distributions uv installs. There is no usable releases
/// API here: a single PBS release carries ~850 assets, so paging the GitHub API
/// times out. Instead we read uv's `download-metadata.json`, a structured index
/// of every PBS build (os/arch/libc/version + direct url), and pick from it.
pub struct Python {
    pub executor_cmd: ExecutorCmd,
}

const METADATA_URL: &str =
    "https://raw.githubusercontent.com/astral-sh/uv/main/crates/uv-python/download-metadata.json";

#[derive(Deserialize)]
struct PyArch {
    family: String,
    variant: Option<String>,
}

#[derive(Deserialize)]
struct PyEntry {
    name: String,
    os: String,
    libc: String,
    arch: PyArch,
    major: u64,
    minor: u64,
    patch: u64,
    // Option (not String + serde default) so an explicit `null` deserializes
    // instead of failing the entry.
    #[serde(default)]
    prerelease: Option<String>,
    variant: Option<String>,
    url: String,
}

/// Map gg's target to the (os, arch-family, libc) triple uv's metadata uses.
/// Returns None for targets PBS doesn't build (armv7, unknown).
fn map_target(target: &Target) -> Option<(&'static str, &'static str, &'static str)> {
    let family = match target.arch {
        Arch::X86_64 => "x86_64",
        Arch::Arm64 => "aarch64",
        // PBS publishes no armv7 builds, and Any can't be resolved to one.
        Arch::Armv7 | Arch::Any => return None,
    };
    let (os, libc) = match (target.os, target.variant) {
        (Os::Windows, _) => ("windows", "none"),
        (Os::Mac, _) => ("darwin", "none"),
        (Os::Linux, Some(Variant::Musl)) => ("linux", "musl"),
        (Os::Linux, _) => ("linux", "gnu"),
        (Os::Any, _) => return None,
    };
    Some((os, family, libc))
}

/// Parse the metadata into entries, tolerating drift: the file is generated on
/// uv's `main` and covers cpython/pypy/graalpy across every platform, so a
/// single new or reshaped entry must not take down all of Python. Parse the
/// outer map as raw JSON, then skip any individual entry that doesn't fit.
fn parse_entries(json: &str) -> Vec<PyEntry> {
    let raw: HashMap<String, serde_json::Value> =
        serde_json::from_str(json).expect("Python metadata was not valid JSON");
    raw.into_values()
        .filter_map(|v| serde_json::from_value::<PyEntry>(v).ok())
        .collect()
}

/// Pick the install-only CPython builds matching the target out of the full
/// metadata. Kept separate from the network fetch so it can be unit tested.
fn select_downloads(entries: Vec<PyEntry>, target: &Target) -> Vec<Download> {
    let Some((want_os, want_family, want_libc)) = map_target(target) else {
        return vec![];
    };

    entries
        .into_iter()
        .filter(|e| e.name == "cpython")
        .filter(|e| e.os == want_os)
        .filter(|e| e.arch.family == want_family)
        // Baseline micro-arch only - the x86_64_v2/v3/v4 builds assume newer CPUs.
        .filter(|e| e.arch.variant.is_none())
        .filter(|e| e.libc == want_libc)
        // Standard build, not freethreaded/debug.
        .filter(|e| e.variant.is_none())
        // Skip betas by default; numbering them as plain X.Y.0 would make a
        // pre-release masquerade as the final stable release.
        .filter(|e| e.prerelease.as_deref().unwrap_or("").is_empty())
        // gg has no zstd support, so only the .tar.gz install-only builds work
        // (PBS ships the bulk of variants as .tar.zst). Require install_only by
        // name so a stray non-install .tar.gz can't sneak in.
        .filter(|e| e.url.contains("install_only") && e.url.ends_with(".tar.gz"))
        .filter_map(|e| {
            let version = format!("{}.{}.{}", e.major, e.minor, e.patch);
            GgVersion::new(&version).map(|v| Download {
                download_url: e.url,
                version: Some(v),
                // Already matched to the target above, so tag as Any and let the
                // framework pick by version (mirrors the node executor).
                os: Some(Os::Any),
                arch: Some(Arch::Any),
                variant: Some(Variant::Any),
                tags: HashSet::new(),
            })
        })
        .collect()
}

async fn get_python_urls(target: &Target) -> Vec<Download> {
    if map_target(target).is_none() {
        return vec![];
    }
    let json = reqwest::get(METADATA_URL)
        .await
        .expect("Failed to fetch Python metadata")
        // Turn an HTTP 404/500 into a clear fetch error instead of feeding an
        // error page into the JSON parser ("metadata not valid JSON").
        .error_for_status()
        .expect("Python metadata request failed")
        .text()
        .await
        .expect("Failed to read Python metadata");
    select_downloads(parse_entries(&json), target)
}

impl Executor for Python {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move { get_python_urls(&input.target).await })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        match &input.target.os {
            Os::Windows => vec![BinPattern::Exact("python.exe".to_string())],
            _ => vec![
                BinPattern::Exact("python3".to_string()),
                BinPattern::Exact("python".to_string()),
            ],
        }
    }

    fn get_name(&self) -> &str {
        "python"
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        // unix: bin/python3, bin/pip; windows: python.exe at root, Scripts/pip.exe
        vec![
            "bin".to_string(),
            ".".to_string(),
            "Scripts".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(os: Os, arch: Arch, variant: Option<Variant>) -> Target {
        Target { os, arch, variant }
    }

    #[test]
    fn test_map_target() {
        assert_eq!(
            map_target(&target(Os::Windows, Arch::X86_64, None)),
            Some(("windows", "x86_64", "none"))
        );
        assert_eq!(
            map_target(&target(Os::Linux, Arch::X86_64, None)),
            Some(("linux", "x86_64", "gnu"))
        );
        assert_eq!(
            map_target(&target(Os::Linux, Arch::X86_64, Some(Variant::Musl))),
            Some(("linux", "x86_64", "musl"))
        );
        assert_eq!(
            map_target(&target(Os::Mac, Arch::Arm64, None)),
            Some(("darwin", "aarch64", "none"))
        );
        // PBS has no armv7 builds.
        assert_eq!(map_target(&target(Os::Linux, Arch::Armv7, None)), None);
    }

    // A trimmed slice of uv's download-metadata.json covering the cases the
    // filter must get right: wanted builds plus every kind we must drop. The
    // last entry is deliberately malformed to exercise tolerant parsing.
    const FIXTURE: &str = r#"{
      "cpython-3.12.3-linux-x86_64-gnu": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":12,"patch":3,"prerelease":"","variant":null,"url":"https://example.com/cpython-3.12.3-x86_64-unknown-linux-gnu-install_only.tar.gz"},
      "cpython-3.11.9-linux-x86_64-gnu": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":11,"patch":9,"prerelease":"","variant":null,"url":"https://example.com/cpython-3.11.9-x86_64-unknown-linux-gnu-install_only.tar.gz"},
      "cpython-3.12.3-linux-x86_64-gnu-full": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":12,"patch":3,"prerelease":"","variant":null,"url":"https://example.com/cpython-3.12.3-unknown-linux-gnu-full.tar.gz"},
      "cpython-3.12.3-linux-x86_64_v3-gnu": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":"v3"},"major":3,"minor":12,"patch":3,"prerelease":"","variant":null,"url":"https://example.com/v3-install_only.tar.gz"},
      "cpython-3.12.3-linux-x86_64-musl": {"name":"cpython","os":"linux","libc":"musl","arch":{"family":"x86_64","variant":null},"major":3,"minor":12,"patch":3,"prerelease":null,"variant":null,"url":"https://example.com/musl-install_only.tar.gz"},
      "cpython-3.13.1-linux-x86_64-gnu-freethreaded": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":13,"patch":1,"prerelease":"","variant":"freethreaded","url":"https://example.com/ft-install_only.tar.gz"},
      "cpython-3.15.0b1-linux-x86_64-gnu": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":15,"patch":0,"prerelease":"b1","variant":null,"url":"https://example.com/beta-install_only.tar.gz"},
      "cpython-3.12.3-linux-x86_64-gnu-zst": {"name":"cpython","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":12,"patch":3,"prerelease":"","variant":null,"url":"https://example.com/zst-install_only.tar.zst"},
      "cpython-3.12.3-windows-x86_64-none": {"name":"cpython","os":"windows","libc":"none","arch":{"family":"x86_64","variant":null},"major":3,"minor":12,"patch":3,"prerelease":"","variant":null,"url":"https://example.com/win-install_only.tar.gz"},
      "pypy-3.10-linux-x86_64-gnu": {"name":"pypy","os":"linux","libc":"gnu","arch":{"family":"x86_64","variant":null},"major":3,"minor":10,"patch":0,"prerelease":"","variant":null,"url":"https://example.com/pypy-install_only.tar.gz"},
      "totally-new-shape": {"name":"cpython","os":"linux","surprise":"a field we don't model and a missing one"}
    }"#;

    fn select(target: &Target) -> Vec<Download> {
        let mut downloads = select_downloads(parse_entries(FIXTURE), target);
        downloads.sort_by(|a, b| a.download_url.cmp(&b.download_url));
        downloads
    }

    #[test]
    fn test_parse_entries_skips_malformed() {
        // The malformed final entry must be dropped, not fail the whole parse.
        let entries = parse_entries(FIXTURE);
        assert_eq!(entries.len(), 10, "should keep the 10 well-formed entries");
    }

    #[test]
    fn test_select_linux_gnu_x86_64_keeps_only_install_only_stable_baseline() {
        let downloads = select(&target(Os::Linux, Arch::X86_64, None));
        let urls: Vec<&str> = downloads.iter().map(|d| d.download_url.as_str()).collect();
        // Only the two stable, baseline, gnu, .tar.gz install_only builds.
        assert_eq!(
            urls,
            vec![
                "https://example.com/cpython-3.11.9-x86_64-unknown-linux-gnu-install_only.tar.gz",
                "https://example.com/cpython-3.12.3-x86_64-unknown-linux-gnu-install_only.tar.gz",
            ]
        );
    }

    #[test]
    fn test_select_excludes_all_the_wrong_builds() {
        let downloads = select(&target(Os::Linux, Arch::X86_64, None));
        let urls: Vec<&str> = downloads.iter().map(|d| d.download_url.as_str()).collect();
        // micro-arch, musl, freethreaded, prerelease, non-install_only .tar.gz,
        // .tar.zst, windows, pypy - each must be gone for a linux-gnu target.
        for bad in ["v3", "musl", "ft", "beta", "full", "zst", "win", "pypy"] {
            assert!(
                !urls.iter().any(|u| u.contains(bad)),
                "should have excluded {}",
                bad
            );
        }
    }

    #[test]
    fn test_select_windows_picks_windows_build() {
        let downloads = select(&target(Os::Windows, Arch::X86_64, None));
        assert_eq!(downloads.len(), 1);
        assert!(downloads[0].download_url.contains("win-install_only"));
    }

    #[test]
    fn test_select_musl_target_picks_musl_build() {
        // This entry also carries `"prerelease": null`, so a pass proves the
        // Option<String> tolerates an explicit null (not just absence/"").
        let downloads = select(&target(Os::Linux, Arch::X86_64, Some(Variant::Musl)));
        assert_eq!(downloads.len(), 1);
        assert!(downloads[0].download_url.contains("musl"));
    }

    #[test]
    fn test_select_version_parsed() {
        let downloads = select(&target(Os::Linux, Arch::X86_64, None));
        let versions: Vec<String> = downloads
            .iter()
            .filter_map(|d| d.version.as_ref().map(|v| v.to_string()))
            .collect();
        assert!(versions.contains(&"3.12.3".to_string()));
        assert!(versions.contains(&"3.11.9".to_string()));
    }
}
