use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::bloody_indiana_jones::BloodyIndianaJones;
use crate::executors::github::GitHub;
use crate::target::{Arch, Os, Target, Variant};
use indicatif::ProgressBar;
use log::{debug, info};
use regex::Regex;
use semver::{Version, VersionReq};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use which::{which_in, which_re_in};

#[derive(PartialEq, Debug, Clone)]
pub struct AppPath {
    pub install_dir: PathBuf,
}

pub struct AppInput {
    pub target: Target,
    pub app_args: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GgVersion(String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GgVersionReq(String);

impl std::fmt::Display for GgVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Locate the version token inside a release tag, dropping any product or `v`
/// prefix: "bun-v1.3.14" -> "1.3.14", "v18" -> "18". It anchors on a dotted
/// `X.Y` version, so a digit buried in the product name ("log4j2-v2.20.0" ->
/// "2.20.0", not "4j2...") can't be mistaken for the version; only when there
/// is no dotted version does it fall back to a bare `[v]N` integer tag.
/// Returns None when nothing version-like is present.
pub fn find_version(tag: &str) -> Option<&str> {
    static DOTTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\d+\.\d+(?:\.\d+)*(?:[-+][0-9A-Za-z][0-9A-Za-z.-]*)?").unwrap());
    static BARE_INT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[vV]?(\d+)$").unwrap());

    if let Some(m) = DOTTED.find(tag) {
        return Some(&tag[m.start()..m.end()]);
    }
    BARE_INT.captures(tag).map(|c| c.get(1).unwrap().as_str())
}

impl GgVersion {
    pub fn to_version(&self) -> Version {
        Version::parse(&self.0).unwrap()
    }

    pub fn new(version: &str) -> Option<Self> {
        // Pull the version out of the tag, dropping any product or `v` prefix
        // (bun ships "bun-v1.3.14"). Anchoring on the version boundary instead
        // of the first digit keeps a digit in the product name from corrupting
        // the parse - a global replace("v", "") mangled both (#293).
        let version = find_version(version)?;
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

impl std::fmt::Display for GgVersionReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GgVersionReq {
    pub fn to_version_req(&self) -> VersionReq {
        VersionReq::parse(&self.0).unwrap()
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
    pub gems: Option<Vec<String>>,
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

impl ExecutorCmd {
    pub fn to_version_selector(&self) -> String {
        let mut selector = String::new();

        if let Some(version) = &self.version {
            selector.push('@');
            selector.push_str(&version.to_string());
        }

        if let Some(distribution) = &self.distribution {
            if !selector.is_empty() {
                selector.push('-');
            } else {
                selector.push_str("@-");
            }
            selector.push_str(distribution);
        }

        for tag in &self.include_tags {
            selector.push('+');
            selector.push_str(tag);
        }

        for tag in &self.exclude_tags {
            selector.push('-');
            selector.push_str(tag);
        }

        selector
    }
}

use crate::tools::get_tool_info;

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

        if let Some(tool_info) = get_tool_info(&executor_cmd.cmd) {
            return (tool_info.factory)(executor_cmd);
        }

        None
    }

    pub fn get_url_matches(&self, urls: &[Download], input: &AppInput) -> Vec<Download> {
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
        input.app_args.clone()
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

pub fn find_jar_file(app_path: &AppPath) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(&app_path.install_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".jar") {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

fn get_executor_app_path(
    _executor: &dyn Executor,
    input: &AppInput,
    path: &str,
) -> Option<AppPath> {
    info!("Trying to find {path}");
    get_app_path(path, input).ok()
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
    } else {
        executor.get_version_req()
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
                .replace("~", "_tilde_")
                .replace("=", "_eq_")
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
        Some(app_path_ok) if app_path_ok.install_dir.exists() => {
            // A populated dir isn't enough: the executor backing a tool can
            // change between gg versions (e.g. codex moving from a GitHub
            // release to an npm package), leaving a dir that no longer holds the
            // binary the current executor expects. Confirm the bin resolves
            // under the cache's own search paths (not $PATH); otherwise drop the
            // stale dir and re-download rather than return a hit that fails at
            // run time.
            let path_vars = bin_path_vars(executor, &app_path_ok);
            let sep = if cfg!(windows) { ";" } else { ":" };
            let all_paths = path_vars.join(sep);
            if resolve_bin_path(&executor.get_bins(input), &path_vars, &all_paths).is_some() {
                return Ok(app_path_ok);
            }
            info!(
                "{name} cache dir {:?} exists but the expected binary is missing; invalidating and re-downloading",
                app_path_ok.install_dir
            );
            let _ = std::fs::remove_dir_all(&app_path_ok.install_dir);
        }
        _ => {
            info!("{name} not found in cache. Download time");
        }
    }

    pb.set_message("Fetching versions".to_string());

    // Drop stale errors from earlier runs, or they get blamed on this tool
    let _ = crate::github_utils::take_github_errors();
    let urls = executor.get_download_urls(input).await;
    pb.set_message(format!("{} versions", &urls.len()));
    debug!("{:?}", urls);

    if urls.is_empty() {
        // Clear the bar first, or it eats the message
        pb.finish_and_clear();
        for reason in crate::github_utils::take_github_errors() {
            eprintln!("{reason}");
        }
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

fn score_filename_match(filename: &str, tool_name: &str, version_re: &Regex) -> u8 {
    if let Some(version_match) = version_re.find(filename) {
        let prefix = &filename[..version_match.start()];
        return if prefix == tool_name {
            0
        } else if prefix.contains(tool_name) || tool_name.contains(prefix) {
            1
        } else {
            2
        };
    }

    if filename.starts_with(&format!("{}-", tool_name))
        || filename.starts_with(&format!("{}.", tool_name))
        || filename.starts_with(&format!("{}_", tool_name))
        || filename == tool_name
    {
        0
    } else if filename.contains(tool_name) {
        1
    } else {
        2
    }
}

/// Non-default build flavors (profiling, debug, older-CPU baseline) we only
/// want when nothing plainer is on offer. A tiebreaker, so `tool-linux-x64.zip`
/// beats `tool-linux-x64-baseline-profile.zip` when both fit the target.
fn variant_noise(filename: &str) -> usize {
    let tokens: Vec<&str> = filename
        .split(|c: char| c == '-' || c == '_' || c == '.')
        .collect();
    ["profile", "debug", "baseline"]
        .iter()
        .filter(|flavor| tokens.contains(flavor))
        .count()
}

fn get_url_matches(urls: &[Download], input: &AppInput, executor: &dyn Executor) -> Vec<Download> {
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
            } else if let Some(u_var) = u.variant {
                if u_var != Variant::Any {
                    return false;
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
            true
        })
        .collect::<Vec<_>>();

    let version_re = Regex::new(r"[-_]v?\d+\.\d+").unwrap();

    urls_match.sort_by(|a, b| {
        let tool_name = executor.get_name().to_lowercase();

        // Split for file name - or else we will get a match from the repo on every file
        let a_filename = a
            .download_url
            .split('/')
            .next_back()
            .unwrap_or("")
            .to_lowercase();
        let b_filename = b
            .download_url
            .split('/')
            .next_back()
            .unwrap_or("")
            .to_lowercase();

        let a_score = score_filename_match(&a_filename, &tool_name, &version_re);
        let b_score = score_filename_match(&b_filename, &tool_name, &version_re);

        let a_score_bucket = if a_score <= 1 { 0 } else { a_score };
        let b_score_bucket = if b_score <= 1 { 0 } else { b_score };

        match a_score_bucket.cmp(&b_score_bucket) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }

        let version_cmp = b
            .version
            .clone()
            .map(|v| v.to_version())
            .cmp(&a.version.clone().map(|v| v.to_version()));

        match version_cmp {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }

        let noise_cmp = variant_noise(&a_filename).cmp(&variant_noise(&b_filename));
        match noise_cmp {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }

        let a_specific = a.os != Some(Os::Any) || a.arch != Some(Arch::Any);
        let b_specific = b.os != Some(Os::Any) || b.arch != Some(Arch::Any);

        match (a_specific, b_specific) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a_score.cmp(&b_score),
        }
    });

    urls_match.into_iter().cloned().collect()
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

fn bin_path_vars(executor: &dyn Executor, app_path: &AppPath) -> Vec<String> {
    executor
        .get_bin_dirs()
        .iter()
        .map(|bin_dir| {
            app_path
                .install_dir
                .join(bin_dir)
                .to_str()
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

/// Resolve the first of `bins` that exists under the given search paths. Shared
/// by `try_run` and `prep`'s cache-hit check so resolution stays in lock-step.
fn resolve_bin_path(bins: &[BinPattern], path_vars: &[String], all_paths: &str) -> Option<PathBuf> {
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
                        which_in(name, Some(all_paths), ".")
                    }
                } else {
                    which_in(name, Some(all_paths), ".")
                }
            }
            BinPattern::Regex(pattern) => {
                if let Ok(regex) = Regex::new(pattern) {
                    which_re_in(regex, Some(all_paths))
                        .ok()
                        .and_then(|mut iter| iter.next())
                        .ok_or(which::Error::CannotFindBinaryPath)
                } else {
                    Err(which::Error::CannotFindBinaryPath)
                }
            }
        };

        if let Ok(bin_path) = bin_path {
            return Some(bin_path);
        }
    }
    None
}

pub async fn try_run(
    input: &AppInput,
    executor: &dyn Executor,
    app_path: AppPath,
    path_vars: Vec<String>,
    env_vars: HashMap<String, String>,
) -> Result<bool, String> {
    let args = executor.customize_args(input, &app_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let paths = env::join_paths(path_vars.clone())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let all_paths = [paths, path_string.to_string()].join(match env::consts::OS {
        "windows" => ";",
        _ => ":",
    });
    info!("PATH: {all_paths}");
    let bins = executor.get_bins(input);
    info!("Trying to find these bins: {:?}", bins);
    if let Some(bin_path) = resolve_bin_path(&bins, &path_vars, &all_paths) {
        info!("Executing: {:?}. With args:{:?}", bin_path, args);
        let mut command = Command::new(&bin_path);

        let child = command
            .env("PATH", all_paths.clone())
            .envs(env_vars.clone())
            .args(args.clone())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;

        let child_handle = Arc::new(Mutex::new(Some(child)));
        let child_handle_clone = Arc::clone(&child_handle);

        let _result = ctrlc::set_handler(move || {
            if let Ok(mut guard) = child_handle_clone.lock() {
                if let Some(ref mut child_process) = *guard {
                    let _ = child_process.kill();
                }
            }
        });

        let res = if let Ok(mut guard) = child_handle.lock() {
            if let Some(ref mut child) = *guard {
                child
                    .wait()
                    .map_err(|_| "Failed to wait for child process")?
                    .success()
            } else {
                false
            }
        } else {
            false
        };

        if !res {
            info!("Unable to execute {}", bin_path.display());
        }
        return Ok(res);
    }
    Err(format!("Error: Unable to find executable for {}. The tool may not be properly installed or the binary name doesn't match expected patterns.", executor.get_name()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github_utils::{detect_arch_from_name, detect_os_from_name};

    // npm-package layout (e.g. codex -> npm_home/bin/codex): a nested bin path,
    // searched via bin_dirs that include "." so base/./npm_home/bin resolves.
    #[cfg(unix)]
    #[test]
    fn test_resolve_bin_path_present_vs_missing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let bins = vec![BinPattern::Exact("npm_home/bin/codex".to_string())];
        let path_vars: Vec<String> = ["bin", ".", "npm_home/bin", "npm_home"]
            .iter()
            .map(|d| root.join(d).to_str().unwrap().to_string())
            .collect();
        let all_paths = path_vars.join(":");

        assert!(
            resolve_bin_path(&bins, &path_vars, &all_paths).is_none(),
            "must report a miss when the expected binary is absent"
        );

        let bin_dir = root.join("npm_home/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("codex");
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            resolve_bin_path(&bins, &path_vars, &all_paths),
            Some(bin),
            "must resolve the nested bin once it is present"
        );
    }

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
    fn test_version_parses_product_prefixed_tag() {
        // bun tags releases "bun-v1.3.14"; the prefix used to make GgVersion
        // return None, so every release got a null version and pinning a
        // version silently fell back to latest (issue #293).
        let v = GgVersion::new("bun-v1.3.14").expect("prefixed tag should parse");
        assert_eq!("1.3.14", v.to_string());

        // A pinned "=1.2.0" must match the bun-v1.2.0 release and nothing else.
        let req = GgVersionReq::new("1.2.0").unwrap();
        let matching = GgVersion::new("bun-v1.2.0").unwrap();
        let other = GgVersion::new("bun-v1.3.14").unwrap();
        assert!(req.to_version_req().matches(&matching.to_version()));
        assert!(!req.to_version_req().matches(&other.to_version()));
    }

    #[test]
    fn test_version_prefix_variants() {
        // Bare "v", generic product prefix, and no prefix all normalise the same.
        assert_eq!("1.2.0", GgVersion::new("v1.2.0").unwrap().to_string());
        assert_eq!("1.2.0", GgVersion::new("1.2.0").unwrap().to_string());
        assert_eq!("2.3.4", GgVersion::new("cli-v2.3.4").unwrap().to_string());
        // Partial and bare-integer tags keep working.
        assert_eq!("22.11.0", GgVersion::new("22.11").unwrap().to_string());
        assert_eq!("18.0.0", GgVersion::new("v18").unwrap().to_string());
        // No digit at all -> no version.
        assert!(GgVersion::new("nightly").is_none());
    }

    #[test]
    fn test_version_ignores_digits_in_product_name() {
        // A digit inside the product name must not be taken for the version:
        // anchoring on the first digit turned "log4j2-v2.20.0" into "4j2..."
        // (no version) and "tool2-v1.2.3" into "2" (wrong version). Anchoring
        // on the dotted version boundary fixes both.
        assert_eq!("2.20.0", GgVersion::new("log4j2-v2.20.0").unwrap().to_string());
        assert_eq!("1.2.3", GgVersion::new("tool2-v1.2.3").unwrap().to_string());
        assert_eq!("5.0.0", GgVersion::new("v2ray-5.0.0").unwrap().to_string());
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

    fn parse_release_assets(text: &str) -> Vec<String> {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| {
                !line.is_empty()
                    && !line.starts_with("sha256:")
                    && !line.contains(" MB ")
                    && !line.contains(" KB ")
                    && !line.contains(" Bytes ")
            })
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_version(filename: &str) -> Option<String> {
        let version_re = Regex::new(r"[-_]v?(\d+\.\d+\.\d+)").ok()?;
        version_re
            .captures(filename)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    fn create_downloads(filenames: &[String]) -> Vec<Download> {
        filenames
            .iter()
            .filter_map(|filename| {
                let name_lower = filename.to_lowercase();

                if name_lower.contains(".orig.tar")
                    || name_lower.contains("-src.")
                    || name_lower.contains("_src.")
                    || name_lower.contains("-source.")
                    || name_lower.contains("_source.")
                {
                    return None;
                }

                let os = detect_os_from_name(filename);
                let arch = detect_arch_from_name(filename);

                let include = (os.is_some() && arch.is_some())
                    || (os.is_none() && arch.is_none())
                    || (os == Some(Os::Windows) && arch.is_none());

                if !include {
                    return None;
                }

                let version = extract_version(filename);

                Some(Download {
                    download_url: format!("https://example.com/releases/{}", filename),
                    version: version.and_then(|v| GgVersion::new(&v)),
                    os: os.or(Some(Os::Any)),
                    arch: arch.or(Some(Arch::Any)),
                    variant: Some(Variant::Any),
                    tags: HashSet::new(),
                })
            })
            .collect()
    }

    fn select_best_download(
        downloads: &[Download],
        tool_name: &str,
        target_os: Os,
        target_arch: Arch,
    ) -> Option<String> {
        let version_re = Regex::new(r"[-_]v?\d+\.\d+").unwrap();
        let tool_name = tool_name.to_lowercase();

        let mut matches: Vec<_> = downloads
            .iter()
            .filter(|d| {
                let os_match = d.os == Some(Os::Any) || d.os == Some(target_os);
                let arch_match = d.arch == Some(Arch::Any) || d.arch == Some(target_arch);
                os_match && arch_match
            })
            .collect();

        matches.sort_by(|a, b| {
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

            let a_score = score_filename_match(&a_filename, &tool_name, &version_re);
            let b_score = score_filename_match(&b_filename, &tool_name, &version_re);

            let a_score_bucket = if a_score <= 1 { 0 } else { a_score };
            let b_score_bucket = if b_score <= 1 { 0 } else { b_score };

            match a_score_bucket.cmp(&b_score_bucket) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }

            let version_cmp = b
                .version
                .clone()
                .map(|v| v.to_version())
                .cmp(&a.version.clone().map(|v| v.to_version()));

            match version_cmp {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }

            let noise_cmp = variant_noise(&a_filename).cmp(&variant_noise(&b_filename));
            match noise_cmp {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }

            let a_specific = a.os != Some(Os::Any) || a.arch != Some(Arch::Any);
            let b_specific = b.os != Some(Os::Any) || b.arch != Some(Arch::Any);

            match (a_specific, b_specific) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_score.cmp(&b_score),
            }
        });

        matches
            .first()
            .and_then(|d| d.download_url.split('/').last())
            .map(|s| s.to_string())
    }

    #[test]
    fn test_sccache_selects_sccache_not_dist() {
        let release_text = r#"
            sccache-dist-v0.12.0-x86_64-unknown-linux-musl.tar.gz
            sha256:4c6f890434cee3521206c2f1f5a772587e5b8f02635a85fe0ade3e83d9b2ec58
            5.08 MB Oct 21
            sccache-v0.12.0-aarch64-apple-darwin.tar.gz
            sha256:4d5281f8760963347b29b9ca4ab1dbde99712c17329619fc9cecba1577ccc8d2
            6.22 MB Oct 21
            sccache-v0.12.0-aarch64-unknown-linux-musl.tar.gz
            sha256:111ddd28fb108cb3e17edb69ab62cefe1dcc97b02e5006ff9c1330f4f2e78368
            8.5 MB Oct 21
            sccache-v0.12.0-x86_64-apple-darwin.tar.gz
            sha256:abc123
            6.5 MB Oct 21
            sccache-v0.12.0-x86_64-unknown-linux-musl.tar.gz
            sha256:def456
            8.5 MB Oct 21
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "sccache", Os::Linux, Arch::X86_64),
            Some("sccache-v0.12.0-x86_64-unknown-linux-musl.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "sccache", Os::Mac, Arch::Arm64),
            Some("sccache-v0.12.0-aarch64-apple-darwin.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "sccache", Os::Linux, Arch::Arm64),
            Some("sccache-v0.12.0-aarch64-unknown-linux-musl.tar.gz".to_string())
        );
    }

    #[test]
    fn test_bun_prefers_plain_over_profile_and_baseline() {
        // bun ships profiling/baseline flavors next to the plain binary; when
        // they all fit the target the plainest one should win, not whatever
        // GitHub happens to list first.
        let release_text = r#"
            bun-linux-x64-baseline-profile.zip
            bun-linux-x64-baseline.zip
            bun-linux-x64-profile.zip
            bun-linux-x64.zip
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "bun", Os::Linux, Arch::X86_64),
            Some("bun-linux-x64.zip".to_string())
        );
    }

    #[test]
    fn test_deno_selection() {
        let release_text = r#"
            deno-aarch64-apple-darwin.zip
            deno-aarch64-unknown-linux-gnu.zip
            deno-x86_64-apple-darwin.zip
            deno-x86_64-pc-windows-msvc.zip
            deno-x86_64-unknown-linux-gnu.zip
            denort-aarch64-apple-darwin.zip
            denort-aarch64-unknown-linux-gnu.zip
            denort-x86_64-apple-darwin.zip
            denort-x86_64-pc-windows-msvc.zip
            denort-x86_64-unknown-linux-gnu.zip
            deno_src.tar.gz
            lib.deno.d.ts
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "deno", Os::Linux, Arch::X86_64),
            Some("deno-x86_64-unknown-linux-gnu.zip".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "deno", Os::Mac, Arch::Arm64),
            Some("deno-aarch64-apple-darwin.zip".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "deno", Os::Windows, Arch::X86_64),
            Some("deno-x86_64-pc-windows-msvc.zip".to_string())
        );
    }

    #[test]
    fn test_caddy_selection() {
        let release_text = r#"
            caddy_2.10.2_linux_amd64.tar.gz
            caddy_2.10.2_linux_arm64.tar.gz
            caddy_2.10.2_linux_armv7.tar.gz
            caddy_2.10.2_mac_amd64.tar.gz
            caddy_2.10.2_mac_arm64.tar.gz
            caddy_2.10.2_windows_amd64.zip
            caddy_2.10.2_windows_arm64.zip
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "caddy", Os::Linux, Arch::X86_64),
            Some("caddy_2.10.2_linux_amd64.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "caddy", Os::Linux, Arch::Arm64),
            Some("caddy_2.10.2_linux_arm64.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "caddy", Os::Windows, Arch::X86_64),
            Some("caddy_2.10.2_windows_amd64.zip".to_string())
        );
    }

    #[test]
    fn test_gh_cli_selection() {
        let release_text = r#"
            gh_2.83.1_linux_amd64.tar.gz
            gh_2.83.1_linux_arm64.tar.gz
            gh_2.83.1_macOS_amd64.zip
            gh_2.83.1_macOS_arm64.zip
            gh_2.83.1_windows_amd64.zip
            gh_2.83.1_windows_arm64.zip
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "cli", Os::Linux, Arch::X86_64),
            Some("gh_2.83.1_linux_amd64.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "cli", Os::Mac, Arch::Arm64),
            Some("gh_2.83.1_macOS_arm64.zip".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "cli", Os::Windows, Arch::X86_64),
            Some("gh_2.83.1_windows_amd64.zip".to_string())
        );
    }

    #[test]
    fn test_just_selection() {
        let release_text = r#"
            just-1.43.1-aarch64-apple-darwin.tar.gz
            just-1.43.1-aarch64-pc-windows-msvc.zip
            just-1.43.1-aarch64-unknown-linux-musl.tar.gz
            just-1.43.1-arm-unknown-linux-musleabihf.tar.gz
            just-1.43.1-armv7-unknown-linux-musleabihf.tar.gz
            just-1.43.1-x86_64-apple-darwin.tar.gz
            just-1.43.1-x86_64-pc-windows-msvc.zip
            just-1.43.1-x86_64-unknown-linux-musl.tar.gz
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "just", Os::Linux, Arch::X86_64),
            Some("just-1.43.1-x86_64-unknown-linux-musl.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "just", Os::Mac, Arch::Arm64),
            Some("just-1.43.1-aarch64-apple-darwin.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "just", Os::Windows, Arch::X86_64),
            Some("just-1.43.1-x86_64-pc-windows-msvc.zip".to_string())
        );
    }

    #[test]
    fn test_fortio_selection() {
        let release_text = r#"
            fortio-linux_amd64-1.73.0.tgz
            fortio-linux_arm64-1.73.0.tgz
            fortio_1.73.0.orig.tar.gz
            fortio_win_1.73.0.zip
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "fortio", Os::Linux, Arch::X86_64),
            Some("fortio-linux_amd64-1.73.0.tgz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "fortio", Os::Linux, Arch::Arm64),
            Some("fortio-linux_arm64-1.73.0.tgz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "fortio", Os::Windows, Arch::Any),
            Some("fortio_win_1.73.0.zip".to_string())
        );
    }

    #[test]
    fn test_fortio_prefers_newer_version_over_better_filename_match() {
        let release_text = r#"
            fortio_0_3_7_linux_x64.gz
            fortio-linux_amd64-1.73.0.tgz
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "fortio", Os::Linux, Arch::X86_64),
            Some("fortio-linux_amd64-1.73.0.tgz".to_string())
        );
    }

    #[test]
    fn test_portable_git_selection() {
        let release_text = r#"
            checksums.txt
            portable-git-linux-x64-v0.6.1.tar.gz
            portable-git-macos-arm64-v0.6.1.tar.gz
            portable-git-macos-x64-v0.6.1.tar.gz
            portable-git-windows-x64-v0.6.1.zip
        "#;

        let filenames = parse_release_assets(release_text);
        let downloads = create_downloads(&filenames);

        assert_eq!(
            select_best_download(&downloads, "portable-git", Os::Linux, Arch::X86_64),
            Some("portable-git-linux-x64-v0.6.1.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "portable-git", Os::Mac, Arch::Arm64),
            Some("portable-git-macos-arm64-v0.6.1.tar.gz".to_string())
        );
        assert_eq!(
            select_best_download(&downloads, "portable-git", Os::Windows, Arch::X86_64),
            Some("portable-git-windows-x64-v0.6.1.zip".to_string())
        );
    }

    #[test]
    fn test_score_filename_match_unit() {
        let version_re = Regex::new(r"[-_]v?\d+\.\d+").unwrap();

        assert_eq!(
            score_filename_match("sccache-v0.12.0-linux.tar.gz", "sccache", &version_re),
            0
        );

        assert_eq!(
            score_filename_match("sccache-dist-v0.12.0-linux.tar.gz", "sccache", &version_re),
            1
        );

        assert_eq!(
            score_filename_match("ripgrep-v1.0.0.tar.gz", "sccache", &version_re),
            2
        );

        assert_eq!(
            score_filename_match("mytool-linux.tar.gz", "mytool", &version_re),
            0
        );
        assert_eq!(
            score_filename_match("othertool-linux.tar.gz", "mytool", &version_re),
            2
        );
    }
}
