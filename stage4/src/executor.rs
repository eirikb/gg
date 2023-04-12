use std::collections::{HashMap, HashSet};
use std::env;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;

use log::{debug, info};
use semver::{Version, VersionReq};

use crate::{download_unpack_and_all_that_stuff, Gradle, Java, NoClap, Node};
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
            os: Some(Os::Any),
            arch: Some(Arch::Any),
            variant: None,
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

impl dyn Executor {
    pub fn new(executor_cmd: ExecutorCmd) -> Option<Box<Self>> {
        match executor_cmd.cmd.as_str() {
            "node" | "npm" | "npx" => Some(Box::new(Node { executor_cmd })),
            "gradle" => Some(Box::new(Gradle { executor_cmd })),
            "java" => Some(Box::new(Java { executor_cmd })),
            _ => None
        }
    }
}

pub trait Executor {
    fn get_executor_cmd(&self) -> &ExecutorCmd;
    fn get_version_req(&self) -> Option<VersionReq>;
    fn get_download_urls<'a>(&'a self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>>;
    fn get_bin(&self, input: &AppInput) -> &str;
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

    fn custom_prep(&self) -> Option<AppPath> {
        None
    }
}

pub async fn prep(executor: &dyn Executor, input: &AppInput) -> Result<AppPath, String> {
    if let Some(app_path) = executor.custom_prep() {
        return Ok(app_path);
    }

    let bin = executor.get_bin(input);
    let version_req = if let Some(ver) = &executor.get_executor_cmd().version {
        Some(ver.clone())
    } else if let Some(ver) = executor.get_version_req() {
        Some(ver)
    } else {
        None
    };
    let version_req_str = &version_req.as_ref().map(|v| v.to_string()).unwrap_or("_star".to_string());
    let path_path = Path::new(executor.get_name()).join(
        executor.get_name().to_string() + &version_req_str.as_str().replace("*", "_star_").replace("^", "_hat_")
    );
    let path = path_path.to_str().unwrap();
    info!( "Trying to find {bin} in {path}");

    let app_path: Result<AppPath, String> = get_app_path(bin, path);

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

    let urls_match = urls.iter().filter(|u| {
        if let Some(u_var) = u.variant {
            if let Some(t_var) = input.target.variant
            {
                if u_var != t_var {
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

        if !(match input.target.os {
            Os::Windows => u.download_url.ends_with(".zip"),
            Os::Linux => u.download_url.ends_with(".tar.gz"),
            Os::Mac => u.download_url.ends_with(".tar.gz"),
            Os::Any => u.download_url.ends_with(".tar.gz")
        }) {
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
                if version_req.matches(version) {
                    return true;
                }
            }
            return false;
        }
        return true;
    }).collect::<Vec<_>>();

    let url = urls_match.first();

    let url_string = if let Some(url) = url {
        &url.download_url
    } else {
        ""
    };

    debug!("{:?}", url_string);

    let cache_path = format!(".cache/{path}");
    download_unpack_and_all_that_stuff(url_string, cache_path.as_str()).await;

    get_app_path(bin, path)
}

pub async fn try_execute(executor: &dyn Executor, input: &AppInput, version_req_map: HashMap<String, Option<VersionReq>>) -> Result<(), String> {
    debug!("Prepping all");

    let mut path_vars: Vec<String> = vec!();
    let mut env_vars: HashMap<String, String> = HashMap::new();
    for (cmd, _) in &version_req_map {
        // if let Some(executor) = cmd_to_executor(cmd.to_string()) {
        //     let res = prep(&*executor, input, &version_req_map).await;
        //     if let Ok(app_path) = res {
        //         path_vars.push(app_path.parent_bin_path());
        //         env_vars.clone_from(&(&*executor).get_env(app_path));
        //     } else if let Err(e) = res {
        //         println!("Unable to prep {}: {}", cmd, e);
        //     }
        // }
    }

    // let app_path = prep(executor, input, &version_req_map).await?.clone();
    // debug!("path is {:?}", app_path);
    // if app_path.bin.exists() {
    //     return if try_run(input, app_path, path_vars, env_vars).await.unwrap() {
    //         Ok(())
    //     } else {
    //         Err("Unable to execute".to_string())
    //     };
    // }
    Ok(())
}

fn get_app_path(bin: &str, path: &str) -> Result<AppPath, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache")
        .join(path);

    let bin_path = path.join(bin);

    Ok(AppPath { app: path, bin: bin_path })
}

pub async fn try_run(input: &AppInput, app_path: AppPath, path_vars: Vec<String>, env_vars: HashMap<String, String>) -> Result<bool, String> {
    let bin_path = app_path.bin.to_str().unwrap_or("");
    info!("Executing: {:?}. With args:{:?}", bin_path,&input.no_clap.app_args);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let parent_bin_path = app_path.parent_bin_path();
    let paths = env::join_paths(path_vars).unwrap().to_str().unwrap().to_string();
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
