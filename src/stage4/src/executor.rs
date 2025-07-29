use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;

use crate::bloody_indiana_jones::BloodyIndianaJones;
use crate::executors::bld::Bld;
use crate::executors::custom_command::CustomCommand;
use crate::executors::flutter::Flutter;
use crate::executors::github::GitHub;
use crate::executors::go::Go;
use crate::executors::gradle::Gradle;
use crate::executors::java::Java;
use crate::executors::jbang::JBangExecutor;
use crate::executors::maven::Maven;
use crate::executors::node::Node;
use crate::executors::openapigenerator::OpenAPIGenerator;
use crate::executors::rat::Rat;
use crate::no_clap::NoClap;
use crate::target::{Arch, Os, Target, Variant};
use indicatif::ProgressBar;
use log::{debug, info};
use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use which::{which_in, which_re_in};

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
        if Version::parse(&version).is_ok() {
            Some(Self(version.to_string()))
        } else {
            None
        }
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
impl AppInput {}

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
        Download {
            download_url,
            version: GgVersion::new(version),
            os: Some(Os::Any),
            arch: Some(Arch::Any),
            variant,
            tags: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorCmd {
    pub cmd: String,
    pub version: Option<GgVersionReq>,
    pub distribution: Option<String>,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutorDep {
    pub name: String,
    pub version: Option<String>,
    pub optional: bool,
}

impl ExecutorDep {
    pub fn new(name: String, version: Option<String>) -> Self {
        Self {
            name,
            version,
            optional: false,
        }
    }

    pub fn optional(name: String, version: Option<String>) -> Self {
        Self {
            name,
            version,
            optional: true,
        }
    }
}

#[cfg(test)]
impl ExecutorCmd {}

fn create_github_executor(
    executor_cmd: ExecutorCmd,
    owner: &str,
    repo: &str,
    deps: Option<Vec<ExecutorDep>>,
    bins: Option<Vec<String>>,
) -> Box<dyn Executor> {
    Box::new(GitHub::new_with_config(
        executor_cmd,
        owner.to_string(),
        repo.to_string(),
        deps,
        bins,
    ))
}

impl dyn Executor {
    pub fn new(executor_cmd: ExecutorCmd) -> Option<Box<Self>> {
        if executor_cmd.cmd.starts_with("gh/") {
            let cmd_clone = executor_cmd.cmd.clone();
            let repo_part = &cmd_clone[3..];
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
            "jbang" => Some(Box::new(JBangExecutor::new(executor_cmd))),
            "maven" | "mvn" => Some(Box::new(Maven { executor_cmd })),
            "openapi" => Some(Box::new(OpenAPIGenerator { executor_cmd })),
            "rat" | "ra" => Some(Box::new(Rat { executor_cmd })),
            "run" => Some(Box::new(CustomCommand { executor_cmd })),
            "bld" => Some(Box::new(Bld::new(executor_cmd))),
            "flutter" | "dart" => Some(Box::new(Flutter { executor_cmd })),
            "deno" => Some(create_github_executor(
                executor_cmd,
                "denoland",
                "deno",
                Some(vec![]),
                Some(vec!["deno".to_string(), "deno.exe".to_string()]),
            )),
            "go" => Some(Box::new(Go { executor_cmd })),
            "caddy" => Some(create_github_executor(
                executor_cmd,
                "caddyserver",
                "caddy",
                Some(vec![]),
                Some(vec!["caddy".to_string(), "caddy.exe".to_string()]),
            )),
            "gh" => Some(create_github_executor(
                executor_cmd,
                "cli",
                "cli",
                Some(vec![ExecutorDep::optional("git".to_string(), None)]),
                Some(vec!["gh".to_string(), "gh.exe".to_string()]),
            )),
            "just" => Some(create_github_executor(
                executor_cmd,
                "casey",
                "just",
                Some(vec![]),
                Some(vec!["just".to_string(), "just.exe".to_string()]),
            )),
            "fortio" => Some(create_github_executor(
                executor_cmd,
                "fortio",
                "fortio",
                Some(vec![]),
                Some(vec!["bin/fortio".to_string(), "fortio.exe".to_string()]),
            )),
            "git" => Some(create_github_executor(
                executor_cmd,
                "eirikb",
                "portable-git",
                Some(vec![]),
                Some(vec!["git".to_string(), "git.exe".to_string()]),
            )),
            _ => None,
        }
    }

    pub fn get_url_matches(&self, urls: &Vec<Download>, input: &AppInput) -> Vec<Download> {
        get_url_matches(urls, input, self)
    }
}

#[derive(Debug, Clone)]
pub enum BinPattern {
    Exact(String),
    Regex(String),
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
    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern>;
    fn get_name(&self) -> &str;
    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move { vec![] })
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

pub fn java_deps<'a>() -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
    Box::pin(async move { vec![ExecutorDep::new("java".to_string(), None)] })
}

fn get_executor_app_path(
    _executor: &dyn Executor,
    input: &AppInput,
    path: &str,
) -> Option<AppPath> {
    info!("Trying to find {path}");
    if let Ok(app_path) = get_app_path(path, input) {
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

    pb.set_message("Fetching versions".to_string());

    let urls = executor.get_download_urls(input).await;
    pb.set_message(format!("{} versions", &urls.len()));
    debug!("{:?}", urls);

    if urls.is_empty() {
        panic!("Did not find any download URL!");
    }

    let urls_match = get_url_matches(&urls, input, executor);

    debug!(
        "Found {} matching URLs for target OS: {:?}, Arch: {:?}",
        urls_match.len(),
        input.target.os,
        input.target.arch
    );

    let url = urls_match.first();

    let url_string = if let Some(url) = url {
        pb.set_prefix(format!(
            "{name} {}",
            url.version.clone().map(|v| v.0).unwrap_or("".to_string())
        ));
        &url.download_url
    } else {
        return Err(format!(
            "No matching download found for OS: {:?}, Arch: {:?}",
            input.target.os, input.target.arch
        ));
    };

    debug!("{:?}", url_string);

    let cache_base_dir = std::env::var("GG_CACHE_DIR").unwrap_or_else(|_| ".cache/gg".to_string());
    let cache_path = format!("{cache_base_dir}/{path}");
    let mut bloody_indiana_jones = BloodyIndianaJones::new_with_cache_dir(
        url_string.to_string(),
        cache_path.clone(),
        &cache_base_dir,
        pb.clone(),
    );
    bloody_indiana_jones.download().await;
    if !executor.post_download(bloody_indiana_jones.file_path.clone()) {
        return Err("Post download failed".to_string());
    }
    bloody_indiana_jones.unpack_and_all_that_stuff().await;
    bloody_indiana_jones.cleanup_download();

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

    get_executor_app_path(executor, input, path).ok_or(format!("Error: Unable to locate {} binary after download. The downloaded package may not contain the expected executable.", executor.get_name()))
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
                    debug!(
                        "Filtering out {:?} - OS mismatch: {:?} != {:?}",
                        u.download_url, os, input.target.os
                    );
                    return false;
                }
            } else {
                debug!("Filtering out {:?} - No OS specified", u.download_url);
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
            debug!(
                "Keeping download: {:?} (OS: {:?}, Arch: {:?}) for target (OS: {:?}, Arch: {:?})",
                u.download_url, u.os, u.arch, input.target.os, input.target.arch
            );
            return true;
        })
        .collect::<Vec<_>>();

    urls_match.sort_by(|a, b| {
        let tool_name = executor.get_name().to_lowercase();

        // Split for file name - or else we will get a match from the repo on every file
        let a_filename = a
            .download_url
            .split('/')
            .last()
            .unwrap_or("")
            .to_lowercase();
        let b_filename = b
            .download_url
            .split('/')
            .last()
            .unwrap_or("")
            .to_lowercase();

        let a_contains_tool = a_filename.contains(&tool_name);
        let b_contains_tool = b_filename.contains(&tool_name);

        match (a_contains_tool, b_contains_tool) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        let a_specific = a.os != Some(Os::Any) || a.arch != Some(Arch::Any);
        let b_specific = b.os != Some(Os::Any) || b.arch != Some(Arch::Any);

        match (a_specific, b_specific) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b
                .version
                .clone()
                .map(|v| v.to_version())
                .cmp(&a.version.clone().map(|v| v.to_version())),
        }
    });

    urls_match.into_iter().map(|d| d.clone()).collect()
}

fn get_app_path(path: &str, _input: &AppInput) -> Result<AppPath, String> {
    let cache_base_dir = std::env::var("GG_CACHE_DIR").unwrap_or_else(|_| ".cache/gg".to_string());
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(cache_base_dir)
        .join(path);

    if path.exists() {
        Ok(AppPath { install_dir: path })
    } else {
        Err("Error: Tool not found in cache. Try running the command again to download and install it.".to_string())
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
    let paths = env::join_paths(path_vars.clone())
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
    info!("Trying to find these bins: {:?}", bins);
    for bin in bins {
        let bin_path = match bin {
            BinPattern::Exact(name) => {
                if name.contains('/') {
                    let path_parts: Vec<&str> = name.split('/').collect();
                    if let Some(binary_name) = path_parts.last() {
                        let custom_paths: Vec<String> = path_vars
                            .iter()
                            .map(|base| {
                                let mut path = PathBuf::from(base);
                                for part in &path_parts[..path_parts.len() - 1] {
                                    path.push(part);
                                }
                                path.to_str().unwrap_or("").to_string()
                            })
                            .collect();
                        let custom_paths_str = custom_paths.join(match env::consts::OS {
                            "windows" => ";",
                            _ => ":",
                        });
                        which_in(binary_name, Some(&custom_paths_str), ".")
                    } else {
                        which_in(&name, Some(&all_paths), ".")
                    }
                } else {
                    which_in(&name, Some(&all_paths), ".")
                }
            }
            BinPattern::Regex(pattern) => {
                if let Ok(regex) = Regex::new(&pattern) {
                    which_re_in(regex, Some(&all_paths))
                        .ok()
                        .and_then(|mut iter| iter.next())
                        .ok_or(which::Error::CannotFindBinaryPath)
                } else {
                    Err(which::Error::CannotFindBinaryPath)
                }
            }
        };

        if let Ok(bin_path) = bin_path {
            info!("Executing: {:?}. With args:{:?}", bin_path, args);
            let mut command = Command::new(&bin_path);
            let res = command
                .env("PATH", all_paths.clone())
                .envs(env_vars.clone())
                .args(args.clone())
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
    Err(format!("Error: Unable to find executable for {}. The tool may not be properly installed or the binary name doesn't match expected patterns.", executor.get_name()))
}
