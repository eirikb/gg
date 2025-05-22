use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;

use crate::bloody_indiana_jones::BloodyIndianaJones;
use crate::executors::caddy::Caddy;
use crate::executors::custom_command::CustomCommand;
use crate::executors::deno::Deno;
use crate::executors::github::GitHub;
use crate::executors::go::Go;
use crate::executors::gradle::Gradle;
use crate::executors::java::Java;
use crate::executors::maven::Maven;
use crate::executors::node::Node;
use crate::executors::openapigenerator::OpenAPIGenerator;
use crate::executors::rat::Rat;
use crate::no_clap::NoClap;
use crate::target::{Arch, Os, Target, Variant};
use indicatif::ProgressBar;
use log::{debug, info};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use which::which_in;

#[derive(PartialEq, Debug, Clone)]
pub struct AppPath {
    pub install_dir: PathBuf,
}

pub struct AppInput {
    pub target: Target,
    pub no_clap: NoClap,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GgVersion(String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GgVersionReq(String);

impl GgVersion {
    pub fn to_version(&self) -> Version {
        Version::parse(&self.0).unwrap()
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn new(version: &str) -> Option<Self> {
        let version = version.replace("v", "");
        let version = version.as_str();
        let parts: Vec<&str> = version.split('.').collect();

        let version = match parts.len() {
            1 => format!("{}.0.0", parts[0]),
            2 => format!("{}.{}.0", parts[0], parts[1]),
            _ => version.to_string(),
        };
        return if Version::parse(&version).is_ok() {
            Some(Self(version.to_string()))
        } else {
            None
        };
    }
}

impl GgVersionReq {
    pub fn to_version_req(&self) -> VersionReq {
        VersionReq::parse(&self.0).unwrap()
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn new(version_req: &str) -> Option<Self> {
        let version_req_with_prefix = if version_req.matches('.').count() == 2
            && !version_req.starts_with('^')
            && !version_req.starts_with('=')
            && !version_req.starts_with('~')
        {
            format!("={}", version_req)
        } else if version_req.matches('.').count() == 1
            && !version_req.starts_with('^')
            && !version_req.starts_with('=')
            && !version_req.starts_with('~')
        {
            format!("~{}", version_req)
        } else {
            version_req.to_string()
        };

        if VersionReq::parse(&version_req_with_prefix).is_ok() {
            Some(Self(version_req_with_prefix))
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GgMeta {
    pub version_req: GgVersionReq,
    pub download: Download,
    pub cmd: ExecutorCmd,
}

#[cfg(test)]
impl AppInput {
    pub fn dummy() -> Self {
        Self {
            target: Target::parse(""),
            no_clap: NoClap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_req_full_semver_exact() {
        let version_req = GgVersionReq::new("22.11.0").unwrap();
        assert_eq!("=22.11.0", version_req.to_string());

        let v1 = GgVersion::new("22.11.0").unwrap();
        assert!(version_req.to_version_req().matches(&v1.to_version()));

        let v2 = GgVersion::new("22.11.1").unwrap();
        assert!(!version_req.to_version_req().matches(&v2.to_version()));

        let v3 = GgVersion::new("22.15.0").unwrap();
        assert!(!version_req.to_version_req().matches(&v3.to_version()));
    }

    #[test]
    fn test_version_req_partial_semver_compatibility() {
        let version_req = GgVersionReq::new("22.11").unwrap();

        let v1 = GgVersion::new("22.11.0").unwrap();
        let v2 = GgVersion::new("22.11.5").unwrap();
        assert!(version_req.to_version_req().matches(&v1.to_version()));
        assert!(version_req.to_version_req().matches(&v2.to_version()));

        let v3 = GgVersion::new("22.12.0").unwrap();
        assert!(!version_req.to_version_req().matches(&v3.to_version()));
    }

    #[test]
    fn test_version_req_with_prefix() {
        let version_req_caret = GgVersionReq::new("^22.11.0").unwrap();
        assert_eq!("^22.11.0", version_req_caret.to_string());

        let version_req_tilde = GgVersionReq::new("~22.11.0").unwrap();
        assert_eq!("~22.11.0", version_req_tilde.to_string());

        let version_req_eq = GgVersionReq::new("=22.11.0").unwrap();
        assert_eq!("=22.11.0", version_req_eq.to_string());
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Download {
    pub version: Option<GgVersion>,
    pub tags: HashSet<String>,
    pub download_url: String,
    pub arch: Option<Arch>,
    pub os: Option<Os>,
    pub variant: Option<Variant>,
}

impl Download {
    pub fn new(download_url: String, version: &str, variant: Option<Variant>) -> Download {
        return Download {
            download_url,
            version: GgVersion::new(version),
            os: Some(Os::Any),
            arch: Some(Arch::Any),
            variant,
            tags: HashSet::new(),
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorCmd {
    pub cmd: String,
    pub version: Option<GgVersionReq>,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
}

#[cfg(test)]
impl ExecutorCmd {
    pub fn dummy() -> Self {
        Self {
            cmd: String::new(),
            version: None,
            include_tags: HashSet::new(),
            exclude_tags: HashSet::new(),
        }
    }
}

impl dyn Executor {
    pub fn new(executor_cmd: ExecutorCmd) -> Option<Box<Self>> {
        if executor_cmd.cmd.starts_with("github/") {
            let cmd_clone = executor_cmd.cmd.clone();
            let repo_part = &cmd_clone[7..];
            if let Some((owner, repo)) = repo_part.split_once('/') {
                return Some(Box::new(GitHub::new(
                    executor_cmd,
                    owner.to_string(),
                    repo.to_string(),
                )));
            }
        }

        match executor_cmd.cmd.as_str() {
            "node" | "npm" | "npx" => Some(Box::new(Node { executor_cmd })),
            "gradle" => Some(Box::new(Gradle::new(executor_cmd))),
            "java" => Some(Box::new(Java { executor_cmd })),
            "maven" | "mvn" => Some(Box::new(Maven { executor_cmd })),
            "openapi" => Some(Box::new(OpenAPIGenerator { executor_cmd })),
            "rat" | "ra" => Some(Box::new(Rat { executor_cmd })),
            "run" => Some(Box::new(CustomCommand { executor_cmd })),
            "deno" => Some(Box::new(Deno { executor_cmd })),
            "go" => Some(Box::new(Go { executor_cmd })),
            "caddy" => Some(Box::new(Caddy { executor_cmd })),
            _ => None,
        }
    }

    pub fn get_url_matches(&self, urls: &Vec<Download>, input: &AppInput) -> Vec<Download> {
        get_url_matches(urls, input, self)
    }
}

pub trait Executor {
    fn get_executor_cmd(&self) -> &ExecutorCmd;
    fn get_version_req(&self) -> Option<VersionReq> {
        None
    }
    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>>;
    fn get_bins(&self, input: &AppInput) -> Vec<String>;
    fn get_name(&self) -> &str;
    fn get_deps(&self) -> Vec<&str> {
        vec![]
    }
    fn get_default_include_tags(&self) -> HashSet<String> {
        HashSet::new()
    }
    fn get_default_exclude_tags(&self) -> HashSet<String> {
        HashSet::new()
    }
    fn get_env(&self, _app_path: &AppPath) -> HashMap<String, String> {
        HashMap::new()
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        vec!["bin".to_string(), ".".to_string()]
    }

    fn customize_args(&self, input: &AppInput, _app_path: &AppPath) -> Vec<String> {
        input.no_clap.app_args.clone()
    }

    fn custom_prep(&self, _input: &AppInput) -> Option<AppPath> {
        None
    }
    fn post_download(&self, _download_file_path: String) -> bool {
        true
    }
    fn post_prep(&self, _cache_path: &str) {}
}

fn get_executor_app_path(
    _executor: &dyn Executor,
    _input: &AppInput,
    path: &str,
) -> Option<AppPath> {
    info!("Trying to find {path}");
    if let Ok(app_path) = get_app_path(path) {
        Some(app_path)
    } else {
        None
    }
}

pub async fn prep(
    executor: &dyn Executor,
    input: &AppInput,
    pb: &ProgressBar,
) -> Result<AppPath, String> {
    if let Some(app_path) = executor.custom_prep(input) {
        return Ok(app_path);
    }

    let executor_cmd = &executor.get_executor_cmd();
    let version_req = if let Some(ver) = &executor_cmd.version {
        Some(ver.to_version_req())
    } else if let Some(ver) = executor.get_version_req() {
        Some(ver)
    } else {
        None
    };
    let version_req_str = &version_req
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or("*".to_string());
    let path_path = Path::new(executor.get_name()).join(
        executor.get_name().to_string()
            + &version_req_str
                .as_str()
                .replace("*", "_star_")
                .replace("^", "_hat_")
            + executor_cmd
                .include_tags
                .iter()
                .map(|t| format!("i{t}"))
                .collect::<Vec<String>>()
                .join("_")
                .as_str()
            + executor_cmd
                .exclude_tags
                .iter()
                .map(|t| format!("e{t}"))
                .collect::<Vec<String>>()
                .join("_")
                .as_str(),
    );
    let path = path_path.to_str().unwrap();

    let app_path = get_executor_app_path(executor, input, path);

    let name = executor.get_name();

    pb.set_prefix(String::from(name));

    match app_path {
        Some(app_path_ok) if app_path_ok.install_dir.exists() => return Ok(app_path_ok),
        _ => {
            info!("{name} not found in cache. Download time");
        }
    }

    pb.set_message(format!("Fetching versions"));

    let urls = executor.get_download_urls(input).await;
    pb.set_message(format!("{} versions", &urls.len()));
    debug!("{:?}", urls);

    if urls.is_empty() {
        panic!("Did not find any download URL!");
    }

    let urls_match = get_url_matches(&urls, input, executor);

    let url = urls_match.first();

    let url_string = if let Some(url) = url {
        pb.set_prefix(format!(
            "{name} {}",
            url.version.clone().map(|v| v.0).unwrap_or("".to_string())
        ));
        &url.download_url
    } else {
        ""
    };

    debug!("{:?}", url_string);

    let cache_path = format!(".cache/gg/{path}");
    let bloody_indiana_jones =
        BloodyIndianaJones::new(url_string.to_string(), cache_path.clone(), pb.clone());
    bloody_indiana_jones.download().await;
    if !executor.post_download(bloody_indiana_jones.file_path.clone()) {
        return Err("Post download failed".to_string());
    }
    bloody_indiana_jones.unpack_and_all_that_stuff().await;

    if let Some(download) = url {
        let download = download;
        let meta = GgMeta {
            download: download.clone(),
            version_req: GgVersionReq(version_req_str.to_string()),
            cmd: executor.get_executor_cmd().clone(),
        };
        let meta_path = Path::new(&cache_path).join("gg-meta.json");
        if let Ok(json) = serde_json::to_string(&meta) {
            if let Ok(mut file) = File::create(meta_path) {
                let _ = file.write_all(json.as_bytes());
            }
        }
    }

    executor.post_prep(cache_path.as_str());

    get_executor_app_path(executor, input, path).ok_or("Binary not found".to_string())
}

fn get_url_matches(
    urls: &Vec<Download>,
    input: &AppInput,
    executor: &dyn Executor,
) -> Vec<Download> {
    let mut urls_match = urls
        .iter()
        .filter(|u| {
            if let Some(t_var) = input.target.variant {
                if let Some(u_var) = u.variant {
                    if u_var != Variant::Any && u_var != t_var {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                if let Some(u_var) = u.variant {
                    if u_var != Variant::Any {
                        return false;
                    }
                }
            }

            if let Some(os) = u.os {
                if os != Os::Any && os != input.target.os {
                    return false;
                }
            } else {
                return false;
            }
            if let Some(arch) = u.arch {
                if arch != Arch::Any && arch != input.target.arch {
                    return false;
                }
            } else {
                return false;
            }

            let cmd = executor.get_executor_cmd();
            for tag in &cmd.include_tags {
                if !u.tags.contains(tag.as_str()) {
                    return false;
                }
            }
            for tag in &executor.get_default_include_tags() {
                if !u.tags.contains(tag.as_str()) {
                    return false;
                }
            }
            for tag in &cmd.exclude_tags {
                if u.tags.contains(tag.as_str()) {
                    return false;
                }
            }
            for tag in executor.get_default_exclude_tags() {
                if u.tags.contains(tag.as_str()) {
                    return false;
                }
            }
            if let Some(version_req) = &cmd.version {
                if let Some(version) = &u.version {
                    if version_req.to_version_req().matches(&version.to_version()) {
                        return true;
                    }
                }
                return false;
            }
            return true;
        })
        .collect::<Vec<_>>();

    urls_match.sort_by(|a, b| {
        b.version
            .clone()
            .map(|v| v.to_version())
            .cmp(&a.version.clone().map(|v| v.to_version()))
    });

    urls_match.into_iter().map(|d| d.clone()).collect()
}

fn get_app_path(path: &str) -> Result<AppPath, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache/gg")
        .join(path);

    if path.exists() {
        Ok(AppPath { install_dir: path })
    } else {
        Err("Binary not found".to_string())
    }
}

pub async fn try_run(
    input: &AppInput,
    executor: &dyn Executor,
    app_path: AppPath,
    path_vars: Vec<String>,
    env_vars: HashMap<String, String>,
) -> Result<bool, String> {
    let args = executor.customize_args(&input, &app_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let paths = env::join_paths(path_vars)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let all_paths = vec![paths, path_string.to_string()].join(match env::consts::OS {
        "windows" => ";",
        _ => ":",
    });
    info!("PATH: {all_paths}");
    let bins = executor.get_bins(&input);
    info!("Trying to find these bins: {}", bins.join(","));
    for bin in bins {
        let bin_paths = which_in(bin, Some(&all_paths), ".");
        if let Ok(bin_path) = bin_paths {
            info!("Executing: {:?}. With args:{:?}", bin_path, args);
            let mut command = Command::new(&bin_path);
            let res = command
                .env("PATH", all_paths)
                .envs(env_vars)
                .args(args)
                .spawn()
                .map_err(|e| e.to_string())?
                .wait()
                .map_err(|_| "eh")?
                .success();
            if !res {
                info!("Unable to execute {}", bin_path.display());
            }
            return Ok(res);
        }
    }
    Err("Binary not found".to_string())
}
