use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use crate::executor::{
    AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep, GgVersion,
};
use crate::github_utils::{create_github_client, detect_arch_from_name, detect_os_from_name};
use crate::target::Os::Windows;
use crate::target::{Arch, Os, Variant};
use log::debug;

pub struct GitHub {
    pub executor_cmd: ExecutorCmd,
    pub owner: String,
    pub repo: String,
    pub predefined_deps: Option<Vec<ExecutorDep>>,
    pub predefined_bins: Option<Vec<String>>,
}

impl GitHub {
    pub fn new(executor_cmd: ExecutorCmd, owner: String, repo: String) -> Self {
        Self {
            executor_cmd,
            owner,
            repo,
            predefined_deps: None,
            predefined_bins: None,
        }
    }

    pub fn new_with_config(
        executor_cmd: ExecutorCmd,
        owner: String,
        repo: String,
        predefined_deps: Option<Vec<ExecutorDep>>,
        predefined_bins: Option<Vec<String>>,
    ) -> Self {
        Self {
            executor_cmd,
            owner,
            repo,
            predefined_deps,
            predefined_bins,
        }
    }

    async fn detect_language_and_deps(&self) -> Vec<ExecutorDep> {
        let octocrab = create_github_client().unwrap();

        if let Ok(repo_info) = octocrab.repos(&self.owner, &self.repo).get().await {
            if let Some(language) = repo_info.language {
                let language_str = language.as_str().unwrap_or("").to_lowercase();
                return match language_str.as_str() {
                    "java" | "kotlin" | "scala" | "clojure" => {
                        vec![ExecutorDep::new("java".to_string(), None)]
                    }
                    "javascript" | "typescript" => vec![ExecutorDep::new("node".to_string(), None)],
                    "go" => vec![],
                    "rust" => vec![],
                    "c" | "c++" | "cpp" => vec![],
                    _ => vec![],
                };
            }
        }

        vec![]
    }

    fn is_likely_binary(name: &str) -> bool {
        let name_lower = name.to_lowercase();

        if name_lower.ends_with(".msi") {
            return false;
        }

        let binary_extensions = [".exe", ".zip", ".tar.gz", ".tgz", ".tar.bz2", ".7z", ".gem"];

        for ext in &binary_extensions {
            if name_lower.contains(ext) {
                return true;
            }
        }

        if (name_lower.contains("linux")
            || name_lower.contains("darwin")
            || name_lower.contains("macos")
            || name_lower.contains("windows"))
            && (name_lower.contains("x64")
                || name_lower.contains("x86")
                || name_lower.contains("arm64")
                || name_lower.contains("aarch64"))
        {
            return true;
        }

        false
    }
}

impl Executor for GitHub {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        let owner = self.owner.clone();
        let repo = self.repo.clone();

        Box::pin(async move {
            let mut downloads: Vec<Download> = vec![];
            // Shh don't tell anyone
            let octocrab = create_github_client().expect("Failed to create GitHub API client");

            let mut page: u32 = 1;
            loop {
                let releases_result = octocrab
                    .repos(&owner, &repo)
                    .releases()
                    .list()
                    .page(page)
                    .per_page(100)
                    .send()
                    .await;

                if let Ok(releases) = releases_result {
                    for release in releases.items {
                        for asset in release.assets {
                            if !Self::is_likely_binary(&asset.name) {
                                continue;
                            }

                            let os = detect_os_from_name(&asset.name);
                            let arch = detect_arch_from_name(&asset.name);

                            debug!("Asset: {} -> OS: {:?}, Arch: {:?}", asset.name, os, arch);

                            if (os.is_some() && arch.is_some())
                                || (os.is_none() && arch.is_none()
                                    || (os == Some(Os::Windows) && arch.is_none()))
                            {
                                debug!(
                                    "Adding download: {} with OS: {:?}, Arch: {:?}",
                                    asset.browser_download_url, os, arch
                                );
                                downloads.push(Download {
                                    download_url: asset.browser_download_url.to_string(),
                                    version: GgVersion::new(release.tag_name.as_str()),
                                    os: os.or(Some(Os::Any)),
                                    arch: arch.or(Some(Arch::Any)),
                                    tags: HashSet::new(),
                                    variant: Some(Variant::Any),
                                });
                            }
                        }
                    }

                    if releases.next.is_none() {
                        break;
                    }
                    page += 1;
                } else {
                    break;
                }
            }
            debug!("Total downloads found: {}", downloads.len());
            downloads
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        if let Some(predefined_bins) = &self.predefined_bins {
            return predefined_bins
                .iter()
                .map(|s| BinPattern::Exact(s.clone()))
                .collect();
        }

        let base_name = &self.repo;

        let mut patterns = vec![
            BinPattern::Exact(match &input.target.os {
                Windows => format!("{}.exe", base_name),
                _ => base_name.clone(),
            }),
            BinPattern::Exact(match &input.target.os {
                Windows => format!("{}.exe", base_name.to_lowercase()),
                _ => base_name.to_lowercase(),
            }),
            BinPattern::Exact(base_name.clone()),
            BinPattern::Exact(base_name.to_lowercase()),
        ];

        match &input.target.os {
            Windows => {
                patterns.push(BinPattern::Regex(r".*\.exe$".to_string()));
            }
            _ => {}
        }

        patterns.push(BinPattern::Regex(r"^[^.]*$".to_string()));

        patterns
    }

    fn get_name(&self) -> &str {
        &self.repo
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            if let Some(predefined_deps) = &self.predefined_deps {
                return predefined_deps.clone();
            }
            self.detect_language_and_deps().await
        })
    }

    fn get_env(&self, app_path: &AppPath) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Ok, so, if this tool uses gems (has .gem files), set gem environment
        let gem_home = app_path.install_dir.join("gem_home");
        if gem_home.exists() {
            let gem_home_str = gem_home.to_string_lossy().to_string();
            env.insert("GEM_HOME".to_string(), gem_home_str.clone());
            env.insert("GEM_PATH".to_string(), gem_home_str);
        }

        env
    }
}
