use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::pin::Pin;

use log::info;
use package_json::PackageJsonManager;
use regex::Regex;
use semver::VersionReq;
use serde::Deserialize;
use serde::Serialize;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::fetch::fetch_json;
use crate::target::{Arch, Os, Target, Variant};

type Root = Vec<Root2>;

#[derive(Serialize, Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum Lts {
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Root2 {
    pub version: String,
    pub files: Vec<String>,
    pub lts: Lts,
}

pub struct Node {
    pub executor_cmd: ExecutorCmd,
    pub npm_package: Option<NpmPackageSpec>,
}

/// A tool distributed as an npm package, executed on a gg-managed Node.js.
/// Gets its own cache dir (named `name`), with the package installed into
/// `npm_home/` by `post_prep` (mirrors the Ruby gems pattern).
#[derive(Clone)]
pub struct NpmPackageSpec {
    /// Tool name (cache dir name), e.g. "gemini-cli"
    pub name: String,
    /// npm package, e.g. "@google/gemini-cli"
    pub package: String,
    /// Executable name the package installs, e.g. "gemini"
    pub bin: String,
}

fn get_package_version() -> Option<Box<VersionReq>> {
    let mut manager = PackageJsonManager::new();
    if manager.locate_closest().is_ok() {
        if let Ok(json) = manager.read_ref() {
            if json.engines.is_some() {
                return Some(Box::new(
                    VersionReq::parse(
                        json.engines
                            .as_ref()
                            .unwrap()
                            .get("node")
                            .unwrap_or(&"".to_string()),
                    )
                    .unwrap_or_default(),
                ));
            }
        }
    }

    if let Ok(nvmrc) = fs::read_to_string(".nvmrc") {
        let nvmrc = Regex::new("^v").unwrap().replace(&nvmrc, "");
        let nvmrc = nvmrc.trim();
        info!("Got version {nvmrc} from .nvmrc");
        if let Ok(ver) = VersionReq::parse(nvmrc) {
            info!("Got parsed version {ver} from .nvmrc");
            return Some(Box::new(ver.clone()));
        }
    }
    None
}

impl Executor for Node {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        get_package_version().map(|v| *v)
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move { get_node_urls(&input.target).await })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        if let Some(spec) = &self.npm_package {
            return match &input.target.os {
                Os::Windows => vec![
                    BinPattern::Exact(format!("npm_home/{}.cmd", spec.bin)),
                    BinPattern::Exact(format!("npm_home/{}", spec.bin)),
                ],
                _ => vec![BinPattern::Exact(format!("npm_home/bin/{}", spec.bin))],
            };
        }
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => match self.executor_cmd.cmd.as_str() {
                    "node" => "node.exe",
                    "npm" => "npm.cmd",
                    _ => "npx.cmd",
                },
                _ => match self.executor_cmd.cmd.as_str() {
                    "node" => "node",
                    "npm" => "npm",
                    _ => "npx",
                },
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        match &self.npm_package {
            Some(spec) => &spec.name,
            None => "node",
        }
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        if self.npm_package.is_some() {
            // node itself + npm-installed bins (unix: npm_home/bin, windows: npm_home)
            vec![
                "bin".to_string(),
                ".".to_string(),
                "npm_home/bin".to_string(),
                "npm_home".to_string(),
            ]
        } else {
            vec!["bin".to_string(), ".".to_string()]
        }
    }

    fn post_prep(&self, cache_path: &str) {
        let Some(spec) = &self.npm_package else {
            return;
        };
        let cache = std::path::Path::new(cache_path);
        let npm_home = cache.join("npm_home");
        let _ = fs::create_dir_all(&npm_home);
        // Keep npm's own download cache (~/.npm) and node-gyp's header cache
        // (~/.cache/node-gyp) inside gg's cache dir, so install is fully
        // self-contained.
        let npm_cache = cache.join("npm_cache");
        let node_gyp_dir = cache.join("node_gyp");

        let (npm, node_bin_dir) = if cfg!(windows) {
            (cache.join("npm.cmd"), cache.to_path_buf())
        } else {
            (cache.join("bin").join("npm"), cache.join("bin"))
        };

        let path_sep = if cfg!(windows) { ";" } else { ":" };
        let path_env = format!(
            "{}{}{}",
            node_bin_dir.display(),
            path_sep,
            std::env::var("PATH").unwrap_or_default()
        );

        info!(
            "Installing npm package {} into {:?}",
            spec.package, npm_home
        );
        let status = std::process::Command::new(&npm)
            .arg("install")
            .arg("-g")
            .arg("--prefix")
            .arg(&npm_home)
            .arg(&spec.package)
            .env("PATH", path_env)
            .env("npm_config_cache", &npm_cache)
            .env("npm_config_devdir", &node_gyp_dir)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => println!("npm install {} failed: {}", spec.package, s),
            Err(e) => println!("npm install {} failed: {}", spec.package, e),
        }
    }
}

async fn unofficial_downloads(target: &Target) -> Vec<Download> {
    download_urls("unofficial-builds.nodejs.org", target).await
}

async fn official_downloads(target: &Target) -> Vec<Download> {
    download_urls("nodejs.org", target).await
}

async fn download_urls(host: &str, target: &Target) -> Vec<Download> {
    let file = match (target.os, target.arch, target.variant) {
        (Os::Windows, Arch::Arm64, _) => "win-arm64-zip",
        (Os::Windows, _, _) => "win-x64-zip",
        (Os::Linux, Arch::Armv7, Some(Variant::Musl)) => "linux-armv7l-musl",
        (Os::Linux, Arch::Arm64, Some(Variant::Musl)) => "linux-arm64-musl",
        (Os::Linux, Arch::X86_64, Some(Variant::Musl)) => "linux-x64-musl",
        (Os::Linux, Arch::Armv7, _) => "linux-armv7l",
        (Os::Linux, Arch::Arm64, _) => "linux-arm64",
        (Os::Mac, Arch::Armv7, _) => "osx-armv7l-tar",
        (Os::Mac, Arch::X86_64, _) => "osx-x64-tar",
        (Os::Mac, Arch::Arm64, _) => "osx-arm64-tar",
        _ => "linux-x64",
    };
    // The musl builds only live on unofficial-builds.nodejs.org, a community box
    // that goes away now and then
    let url = format!("https://{host}/download/release/index.json");
    let root: Root = match fetch_json(&url).await {
        Some(root) => root,
        None => return vec![],
    };
    downloads_from_index(&root, host, file)
}

fn downloads_from_index(root: &Root, host: &str, file: &str) -> Vec<Download> {
    root.iter().filter(|r|
        r.files.contains(&file.to_string())
    ).filter_map(|r| {
        let version_string = r.version.as_str();
        let version = GgVersion::new(version_string)?;
        // nodejs.org lists alphas next to the releases, and v27.0.0-alpha.0.0.0
        // beats every v26 when nothing is pinned
        if !version.to_version().pre.is_empty() {
            return None;
        }
        let lts = matches!(r.lts, Lts::String(_));
        let file_fix = if file.ends_with("-zip") {
            file.replace("-zip", ".zip")
        } else {
            file.to_string() + ".tar.gz"
        }.replace("osx", "darwin").replace("-tar", "");

        let tags: HashSet<String> = if lts {
            ["lts".to_string()].iter().cloned().collect()
        } else {
            HashSet::new()
        };
        Some(Download {
            download_url: format!("https://{host}/download/release/{version_string}/node-{version_string}-{file_fix}"),
            version: Some(version),
            tags,
            // Arch and Os are mapped by target Arch/Os
            arch: Some(Arch::Any),
            os: Some(Os::Any),
            variant: Some(Variant::Any),
        })
    }).collect()
}

async fn get_node_urls(target: &Target) -> Vec<Download> {
    match (target.os, target.arch, target.variant) {
        (Os::Linux, _, Some(Variant::Musl)) => unofficial_downloads(target).await,
        (Os::Windows, Arch::Arm64, _) => unofficial_downloads(target).await,
        _ => official_downloads(target).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from nodejs.org on 2026-11-11, plus an rc and a version that won't parse
    const INDEX: &str = r#"[
      {"version":"v27.0.0-alpha.0.0.0","date":"2026-10-29","files":["linux-x64","win-x64-zip"],"npm":"12.1.0","v8":"15.6.75.25","lts":false,"security":false},
      {"version":"v26.13.1","date":"2026-11-10","files":["linux-x64","win-x64-zip"],"npm":"11.20.0","v8":"14.6.202.34","lts":"Lithium","security":true},
      {"version":"v26.14.0-rc.1","date":"2026-11-05","files":["linux-x64","win-x64-zip"],"npm":"11.20.0","v8":"14.6.202.34","lts":false,"security":false},
      {"version":"latest","date":"2026-11-05","files":["linux-x64","win-x64-zip"],"npm":"11.20.0","v8":"14.6.202.34","lts":false,"security":false},
      {"version":"v25.9.0","date":"2026-04-01","files":["linux-x64"],"npm":"11.6.0","v8":"14.1.146.11","lts":false,"security":false},
      {"version":"v24.12.0","date":"2026-03-01","files":["win-x64-zip"],"npm":"11.6.0","v8":"13.6.233.10","lts":"Krypton","security":false}
    ]"#;

    #[test]
    fn test_index_skips_prereleases() {
        let root: Root = serde_json::from_str(INDEX).unwrap();
        let downloads = downloads_from_index(&root, "nodejs.org", "linux-x64");
        let got: Vec<(String, bool)> = downloads
            .iter()
            .map(|d| {
                (
                    d.version.as_ref().unwrap().to_string(),
                    d.tags.contains("lts"),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![("26.13.1".to_string(), true), ("25.9.0".to_string(), false)]
        );
        assert_eq!(
            downloads[0].download_url,
            "https://nodejs.org/download/release/v26.13.1/node-v26.13.1-linux-x64.tar.gz"
        );
    }

    #[test]
    fn test_index_unofficial_zip_skips_prereleases() {
        let root: Root = serde_json::from_str(INDEX).unwrap();
        let downloads = downloads_from_index(&root, "unofficial-builds.nodejs.org", "win-x64-zip");
        let urls: Vec<&str> = downloads.iter().map(|d| d.download_url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://unofficial-builds.nodejs.org/download/release/v26.13.1/node-v26.13.1-win-x64.zip",
                "https://unofficial-builds.nodejs.org/download/release/v24.12.0/node-v24.12.0-win-x64.zip",
            ]
        );
    }
}
