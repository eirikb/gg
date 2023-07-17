use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;

use indicatif::ProgressBar;
use log::{debug, info};
use semver::{Version, VersionReq};

use crate::{download_unpack_and_all_that_stuff, Gradle, Java, NoClap, Node};
use crate::custom_command::CustomCommand;
use crate::maven::Maven;
use crate::openapigenerator::OpenAPIGenerator;
use crate::rat::Rat;
use crate::target::{Arch, Os, Target, Variant};

#[derive(PartialEq, Debug, Clone)]
pub struct AppPath {
    pub app: PathBuf,
    pub bin: PathBuf,
}

impl AppPath {
    pub(crate) fn parent_bin_path(&self) -> String {
        self.bin.parent().unwrap_or(Path::new("/")).to_str().unwrap_or("").to_string()
    }
}

pub struct AppInput {
    pub target: Target,
    pub no_clap: NoClap,
}

#[cfg(test)]
impl AppInput {
    pub fn dummy() -> Self {
        Self { target: Target::parse(""), no_clap: NoClap::new() }
    }
}

#[derive(Debug, Clone)]
pub struct Download {
    pub version: Option<Version>,
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
            version: Version::parse(version).ok(),
            os: Some(Os::Any),
            arch: Some(Arch::Any),
            variant,
            tags: HashSet::new(),
        };
    }
}

#[derive(Debug, Clone)]
pub struct ExecutorCmd {
    pub cmd: String,
    pub version: Option<VersionReq>,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
}

#[cfg(test)]
impl ExecutorCmd {
    pub fn dummy() -> Self {
        Self { cmd: String::new(), version: None, include_tags: HashSet::new(), exclude_tags: HashSet::new() }
    }
}

impl dyn Executor {
    pub fn new(executor_cmd: ExecutorCmd) -> Option<Box<Self>> {
        match executor_cmd.cmd.as_str() {
            "node" | "npm" | "npx" => Some(Box::new(Node { executor_cmd })),
            "gradle" => Some(Box::new(Gradle { executor_cmd })),
            "java" => Some(Box::new(Java { executor_cmd })),
            "maven" | "mvn" => Some(Box::new(Maven { executor_cmd })),
            "openapi" => Some(Box::new(OpenAPIGenerator { executor_cmd })),
            "rat" | "ra" => Some(Box::new(Rat { executor_cmd })),
            "run" => Some(Box::new(CustomCommand { executor_cmd })),
            _ => None,
        }
    }
}

pub trait Executor {
    fn get_executor_cmd(&self) -> &ExecutorCmd;
    fn get_version_req(&self) -> Option<VersionReq> {
        None
    }
    fn get_download_urls<'a>(&'a self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>>;
    fn get_bin(&self, input: &AppInput) -> Vec<&str>;
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
    fn get_env(&self, _app_path: AppPath) -> HashMap<String, String> {
        HashMap::new()
    }
    fn get_custom_bin_path(&self, _paths: &str) -> Option<String> { None }
    fn customize_args(&self, input: &AppInput, _app_path: &AppPath) -> Vec<String> {
        input.no_clap.app_args.clone()
    }

    fn custom_prep(&self, _input: &AppInput) -> Option<AppPath> {
        None
    }
    fn post_prep(&self, _cache_path: &str) {}
}

fn get_executor_app_path(executor: &dyn Executor, input: &AppInput, path: &str) -> Option<AppPath> {
    let bins = executor.get_bin(input);
    let bin = bins.join(",");
    info!( "Trying to find {bin} in {path}");
    if let Some(p) = bins.iter().map(|bin| {
        if let Ok(app_path) = get_app_path(bin, path) {
            Some(app_path)
        } else {
            None
        }
    }).find(|p| p.is_some()) {
        p
    } else {
        None
    }
}

pub async fn prep(executor: &dyn Executor, input: &AppInput, pb: &ProgressBar) -> Result<AppPath, String> {
    if let Some(app_path) = executor.custom_prep(input) {
        return Ok(app_path);
    }

    let executor_cmd = &executor.get_executor_cmd();
    let version_req = if let Some(ver) = &executor_cmd.version {
        Some(ver.clone())
    } else if let Some(ver) = executor.get_version_req() {
        Some(ver)
    } else {
        None
    };
    let version_req_str = &version_req.as_ref().map(|v| v.to_string()).unwrap_or("*".to_string());
    let path_path = Path::new(executor.get_name()).join(
        executor.get_name().to_string() + &version_req_str.as_str().replace("*", "_star_").replace("^", "_hat_")
            + executor_cmd.include_tags.iter().map(|t| format!("i{t}")).collect::<Vec<String>>().join("_").as_str()
            + executor_cmd.exclude_tags.iter().map(|t| format!("e{t}")).collect::<Vec<String>>().join("_").as_str()
    );
    let path = path_path.to_str().unwrap();

    let app_path = get_executor_app_path(executor, input, path);

    let name = executor.get_name();

    pb.set_prefix(String::from(name));

    match app_path {
        Some(app_path_ok) if app_path_ok.bin.exists() => return Ok(app_path_ok),
        _ => {
            info!("App {name} not found in cache. Download time");
        }
    }

    pb.set_message(format!("Fetching versions"));

    let urls = executor.get_download_urls(input).await;
    pb.set_message(format!("{} versions", &urls.len()));
    debug!( "{:?}", urls);

    if urls.is_empty() {
        panic!("Did not find any download URL!");
    }

    let mut urls_match = urls.iter().filter(|u| {
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

        // SKIP?!
        // if let Some(os) = u.os {
        //     if os != Os::Any && !(match input.target.os {
        //         Os::Windows => u.download_url.ends_with(".zip"),
        //         Os::Linux => u.download_url.ends_with(".tar.gz"),
        //         Os::Mac => u.download_url.ends_with(".tar.gz"),
        //         Os::Any => u.download_url.ends_with(".tar.gz")
        //     }) {
        //         return false;
        //     }
        // }

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
                if version_req.matches(version) {
                    return true;
                }
            }
            return false;
        }
        return true;
    }).collect::<Vec<_>>();

    urls_match.sort_by(|a, b| b.version.cmp(&a.version));

    let url = urls_match.first();

    let url_string = if let Some(url) = url {
        pb.set_prefix(format!("{name} {}", url.version.clone().map(|v| v.to_string()).unwrap_or("".to_string())));
        &url.download_url
    } else {
        ""
    };

    debug!("{:?}", url_string);

    let cache_path = format!(".cache/{path}");
    download_unpack_and_all_that_stuff(url_string, cache_path.as_str(), pb).await;

    executor.post_prep(cache_path.as_str());

    get_executor_app_path(executor, input, path).ok_or("Binary not found".to_string())
}

fn get_app_path(bin: &str, path: &str) -> Result<AppPath, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache")
        .join(path);

    let bin_path = path.join(bin);

    if bin_path.exists() {
        Ok(AppPath { app: path, bin: bin_path })
    } else {
        Err("Binary not found".to_string())
    }
}

pub async fn try_run(input: &AppInput, executor: &dyn Executor, app_path: AppPath, path_vars: Vec<String>, env_vars: HashMap<String, String>) -> Result<bool, String> {
    let args = executor.customize_args(&input, &app_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let parent_bin_path = app_path.parent_bin_path();
    let paths = env::join_paths(path_vars).unwrap().to_str().unwrap().to_string();
    let all_paths = vec!(parent_bin_path, paths, path_string.to_string()).join(":");
    let bin_path = if let Some(bin_path) = executor.get_custom_bin_path(all_paths.as_str()) {
        bin_path
    } else {
        app_path.bin.to_str().unwrap_or("").to_string()
    };
    info!("Executing: {:?}. With args:{:?}", bin_path, args);
    debug!("PATH: {all_paths}");
    let mut command = Command::new(&bin_path);
    let res = command
        .env("PATH", all_paths)
        .envs(env_vars)
        .args(args)
        .spawn().map_err(|e| e.to_string())?.wait().map_err(|_| "eh")?.success();
    if !res {
        info!("Unable to execute {bin_path}");
    }

    Ok(res)
}
