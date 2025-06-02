use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::executor::{
    AppInput, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep, GgVersion,
};
use crate::target::Os::Windows;
use crate::target::{Arch, Os, Variant};

pub struct GitHub {
    pub executor_cmd: ExecutorCmd,
    pub owner: String,
    pub repo: String,
    pub predefined_deps: Option<Vec<String>>,
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
        predefined_deps: Option<Vec<String>>,
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
        let octocrab = octocrab::Octocrab::builder()
            .base_uri("https://ghapi.ggcmd.io/")
            .unwrap()
            .build()
            .unwrap();

        if let Ok(repo_info) = octocrab.repos(&self.owner, &self.repo).get().await {
            if let Some(language) = repo_info.language {
                let language_str = language.as_str().unwrap_or("").to_lowercase();
                return match language_str.as_str() {
                    "java" | "kotlin" | "scala" | "clojure" => vec![ExecutorDep {
                        name: "java".to_string(),
                        version: None,
                    }],
                    "javascript" | "typescript" => vec![ExecutorDep {
                        name: "node".to_string(),
                        version: None,
                    }],
                    "go" => vec![],
                    "rust" => vec![],
                    "c" | "c++" | "cpp" => vec![],
                    _ => vec![],
                };
            }
        }

        vec![]
    }

    fn detect_os_from_name(name: &str) -> Option<Os> {
        let name_lower = name.to_lowercase();
        if name_lower.contains("darwin")
            || name_lower.contains("macos")
            || name_lower.contains("apple")
        {
            Some(Os::Mac)
        } else if name_lower.contains("windows")
            || name_lower.contains("win")
            || name_lower.contains(".exe")
        {
            Some(Windows)
        } else if name_lower.contains("linux") {
            Some(Os::Linux)
        } else {
            None
        }
    }

    fn detect_arch_from_name(name: &str) -> Option<Arch> {
        let name_lower = name.to_lowercase();
        if name_lower.contains("x86_64")
            || name_lower.contains("amd64")
            || name_lower.contains("x64")
        {
            Some(Arch::X86_64)
        } else if name_lower.contains("arm64") || name_lower.contains("aarch64") {
            Some(Arch::Arm64)
        } else if name_lower.contains("armv7") || name_lower.contains("arm") {
            Some(Arch::Armv7)
        } else {
            None
        }
    }

    fn is_likely_binary(name: &str) -> bool {
        let name_lower = name.to_lowercase();

        let binary_extensions = [".exe", ".zip", ".tar.gz", ".tgz", ".tar.bz2"];

        for ext in &binary_extensions {
            if name_lower.contains(ext) {
                return true;
            }
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
            let octocrab = octocrab::Octocrab::builder()
                .base_uri("https://ghapi.ggcmd.io/")
                .unwrap()
                .build()
                .expect("Failed to create GitHub API client");

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

                            let os = Self::detect_os_from_name(&asset.name);
                            let arch = Self::detect_arch_from_name(&asset.name);

                            if (os.is_some() && arch.is_some()) || (os.is_none() && arch.is_none())
                            {
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
                return predefined_deps
                    .iter()
                    .map(|s| ExecutorDep {
                        name: s.clone(),
                        version: None,
                    })
                    .collect();
            }
            self.detect_language_and_deps().await
        })
    }
}
