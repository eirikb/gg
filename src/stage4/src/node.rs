use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::pin::Pin;

use log::info;
use package_json::PackageJsonManager;
use regex::Regex;
use semver::{Version, VersionReq};
use serde::Deserialize;
use serde::Serialize;

use crate::executor::{AppInput, Download, Executor, ExecutorCmd};
use crate::target::{Arch, Os, Target, Variant};

type Root = Vec<Root2>;

#[derive(Serialize, Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum LTS {
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
    pub lts: LTS,
    pub security: bool,
}

pub struct Node {
    pub executor_cmd: ExecutorCmd,
}

fn get_package_version() -> Option<Box<VersionReq>> {
    let mut manager = PackageJsonManager::new();
    if manager.locate_closest().is_ok() {
        if let Ok(json) = manager.read_ref() {
            if json.engines.is_some() {
                return Some(Box::new(VersionReq::parse(json.clone().engines.clone().unwrap().get("node").unwrap_or(&"".to_string())).unwrap_or(VersionReq::default())));
            }
        }
    }


    if let Ok(nvmrc) = fs::read_to_string(".nvmrc") {
        let nvmrc = Regex::new("^v").unwrap().replace(&nvmrc, "");
        let nvmrc = nvmrc.trim();
        info!("Got version {nvmrc} from .nvmrc");
        if let Ok(ver) = VersionReq::parse(&nvmrc) {
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
        if let Some(v) = get_package_version() {
            Some(*v)
        } else {
            None
        }
    }

    fn get_download_urls<'a>(&self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move { get_node_urls(&input.target).await })
    }

    fn get_bin(&self, input: &AppInput) -> &str {
        match &input.target.os {
            Os::Windows => match self.executor_cmd.cmd.as_str() {
                "node" => "node.exe",
                "npm" => "npm.cmd",
                _ => "npx.cmd",
            },
            _ => match self.executor_cmd.cmd.as_str() {
                "node" => "bin/node",
                "npm" => "bin/npm",
                _ => "bin/npx"
            }
        }
    }

    fn get_name(&self) -> &str {
        "node"
    }
}

async fn unofficial_downloads(target: &Target) -> Vec<Download> {
    return download_urls("unofficial-builds.nodejs.org", target).await;
}

async fn official_downloads(target: &Target) -> Vec<Download> {
    return download_urls("nodejs.org", target).await;
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
    let json = reqwest::get(format!("https://{host}/download/release/index.json")).await.unwrap().text().await.unwrap();
    let root: Root = serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");

    root.iter().filter(|r|
        r.files.contains(&file.to_string())
    ).map(|r| {
        let lts = match r.lts {
            LTS::String(_) => true,
            _ => false
        };
        let file_fix = if file.ends_with("-zip") {
            file.replace("-zip", ".zip")
        } else {
            file.to_string() + ".tar.gz"
        }.replace("osx", "darwin").replace("-tar", "");

        let version = r.clone().version;
        let tags: HashSet<String> = if lts {
            ["lts".to_string()].iter().cloned().collect()
        } else {
            HashSet::new()
        };
        let string = version.replace("v", "");
        let result = Version::parse(string.as_str());
        return Download {
            download_url: format!("https://{host}/download/release/{version}/node-{version}-{file_fix}"),
            version: result.ok(),
            tags,
            // Arch and Os are mapped by target Arch/Os
            arch: Some(Arch::Any),
            os: Some(Os::Any),
            variant: Some(Variant::Any),
        };
    }).collect()
}

async fn get_node_urls(target: &Target) -> Vec<Download> {
    match (target.os, target.arch, target.variant) {
        (Os::Linux, _, Some(Variant::Musl)) => unofficial_downloads(target).await,
        (Os::Windows, Arch::Arm64, _) => unofficial_downloads(target).await,
        _ => official_downloads(target).await
    }
}
