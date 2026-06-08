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
    pub date: String,
    pub files: Vec<String>,
    pub npm: Option<String>,
    pub v8: String,
    pub uv: Option<String>,
    pub zlib: Option<String>,
    pub openssl: Option<String>,
    pub modules: Option<String>,
    pub lts: Lts,
    pub security: bool,
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
    let json = reqwest::get(format!("https://{host}/download/release/index.json"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let root: Root = serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");

    root.iter().filter(|r|
        r.files.contains(&file.to_string())
    ).map(|r| {
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
        let version_string = r.version.as_str();
        let version = GgVersion::new(version_string);
        Download {
            download_url: format!("https://{host}/download/release/{version_string}/node-{version_string}-{file_fix}"),
            version,
            tags,
            // Arch and Os are mapped by target Arch/Os
            arch: Some(Arch::Any),
            os: Some(Os::Any),
            variant: Some(Variant::Any),
        }
    }).collect()
}

async fn get_node_urls(target: &Target) -> Vec<Download> {
    match (target.os, target.arch, target.variant) {
        (Os::Linux, _, Some(Variant::Musl)) => unofficial_downloads(target).await,
        (Os::Windows, Arch::Arm64, _) => unofficial_downloads(target).await,
        _ => official_downloads(target).await,
    }
}
