use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use log::{debug, info};
use regex::Regex;
use semver::{Version, VersionReq};
use crate::{cmd_to_executor, download_unpack_and_all_that_stuff, NoClap};
use crate::target::{Arch, Os, Target, Variant};
use crate::version::{GGVersion, GGVersionReq};

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
    pub fn new(download_url: String, version: &str) -> Download {
        return Download {
            download_url,
            version: Version::parse(version).ok(),
            os: None,
            arch: None,
            variant: None,
            tags: HashSet::new(),
        };
    }
}

pub trait Executor {
    fn get_version_req(&self) -> Option<VersionReq>;
    fn get_download_urls<'a>(&'a self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>>;
    fn get_bin(&self, input: &AppInput) -> &str;
    fn get_name(&self) -> &str;
    fn get_deps(&self) -> Vec<&str> {
        vec![]
    }
    fn get_env(&self, _app_path: AppPath) -> HashMap<String, String> {
        HashMap::new()
    }

    fn custom_prep(&self) -> Option<AppPath> {
        None
    }
}

fn get_version_req(input: &AppInput, executor: &dyn Executor, version_req_map: &HashMap<String, Option<VersionReq>>) -> Option<VersionReq> {
    let version_req = executor.get_version_req();
    return if let Some(version_req) = version_req {
        Some(version_req)
    } else {
        if let Some(cmd) = &input.no_clap.cmd {
            if let Some(v_q) = version_req_map.get(cmd) {
                return v_q.clone();
            }
        };
        None
    };
}

pub async fn prep(executor: &dyn Executor, input: &AppInput, version_req_map: &HashMap<String, Option<VersionReq>>) -> Result<AppPath, String> {
    if let Some(app_path) = executor.custom_prep() {
        return Ok(app_path);
    }

    let bin = executor.get_bin(input);
    let path = (executor.get_name().to_string() + executor.get_version_req().unwrap_or(VersionReq::default()).to_string().as_str()).replace("*", "_star_").replace("^", "_hat_");
    info!( "Trying to find {bin} in {path}");

    let app_path: Result<AppPath, String> = get_app_path(bin, path.as_str());

    match app_path {
        Ok(app_path_ok) if app_path_ok.bin.exists() => return Ok(app_path_ok),
        _ => {
            info!("App {bin} not found in cache. Download time");
        }
    }

    let urls = executor.get_download_urls(input).await;
    debug!( "{:?}", urls);

    if urls.is_empty() {
        panic!("Did not find any download URL!");
    }

    let version_req = get_version_req(input, executor, version_req_map);
    let urls_match_target_and_tags = urls.iter().filter(|u| {
        if let Some(uvar) = u.variant {
            if let Some(tvar) = input.target.variant {
                if uvar != tvar {
                    return false;
                }
            }
        }
        if u.os.is_some() && u.os.unwrap() != input.target.os {
            return false;
        }
        if u.arch.is_some() && u.arch.unwrap() != input.target.arch {
            return false;
        }
        for tag in &u.tags {
            if !u.tags.contains(tag) {
                return false;
            }
        }
        return true;
    }).collect::<Vec<_>>();

    let url = urls_match_target_and_tags.iter().find(|u| {
        if let Some(version) = &u.version {
            if let Some(version_req) = &version_req {
                if version_req.matches(version) {
                    return true;
                }
            }
        }
        return true;
    });

    let url_string = if let Some(url) = url {
        &url.download_url
    } else {
        &urls[0].download_url
    };

    debug!("{:?}", url_string);

    let cache_path = format!(".cache/{path}");
    download_unpack_and_all_that_stuff(url_string, cache_path.as_str()).await;

    get_app_path(bin, path.as_str())
}

pub async fn try_execute(executor: &dyn Executor, input: &AppInput, version_req_map: HashMap<String, Option<VersionReq>>) -> Result<(), String> {
    debug!("Prepping all");

    let mut path_vars: Vec<String> = vec!();
    let mut env_vars: HashMap<String, String> = HashMap::new();
    for (cmd, _) in &version_req_map {
        if let Some(executor) = cmd_to_executor(cmd.to_string()) {
            let res = prep(&*executor, input, &version_req_map).await;
            if let Ok(app_path) = res {
                path_vars.push(app_path.parent_bin_path());
                env_vars.clone_from(&(&*executor).get_env(app_path));
            } else if let Err(e) = res {
                println!("Unable to prep {}: {}", cmd, e);
            }
        }
    }

    let app_path = prep(executor, input, &version_req_map).await?.clone();
    debug!("path is {:?}", app_path);
    if app_path.bin.exists() {
        return if try_run(input, app_path, path_vars, env_vars).await.unwrap() {
            Ok(())
        } else {
            Err("Unable to execute".to_string())
        };
    }
    Ok(())
}

fn get_app_path(bin: &str, path: &str) -> Result<AppPath, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache")
        .join(path);

    let dir_entry = path
        .read_dir()
        .map_err(|_| ".cache not found")?
        .next()
        .ok_or("")?;

    let app_path = dir_entry
        .map_err(|_| "app dir not found")?
        .path();


    let bin_path = app_path.join(bin);

    Ok(AppPath { app: app_path, bin: bin_path })
}

async fn try_run(input: &AppInput, app_path: AppPath, path_vars: Vec<String>, env_vars: HashMap<String, String>) -> Result<bool, String> {
    let bin_path = app_path.bin.to_str().unwrap_or("");
    info!("Executing: {:?}", bin_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let parent_bin_path = app_path.parent_bin_path();
    let paths = path_vars.join(":");
    let all_paths = vec!(parent_bin_path, paths, path_string.to_string()).join(":");
    debug!("PATH: {all_paths}");
    let mut command = Command::new(&bin_path);
    let res = command
        .env("PATH", all_paths)
        .envs(env_vars)
        .args(&input.no_clap.app_args)
        .spawn().map_err(|e| e.to_string())?.wait().map_err(|_| "eh")?.success();
    if !res {
        info!("Unable to execute {bin_path}");
    }

    Ok(res)
}
