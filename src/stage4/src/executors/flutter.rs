use crate::executor::{AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os};
use semver::VersionReq;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Deserialize)]
struct PubspecEnvironment {
    flutter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Pubspec {
    environment: Option<PubspecEnvironment>,
}

pub struct Flutter {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Flutter {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        if let Ok(content) = std::fs::read_to_string("pubspec.yaml") {
            if let Ok(pubspec) = serde_yml::from_str::<Pubspec>(&content) {
                if let Some(environment) = pubspec.environment {
                    if let Some(flutter_version) = environment.flutter {
                        if let Ok(version_req) = VersionReq::parse(&flutter_version) {
                            return Some(version_req);
                        }
                    }
                }
            }
        }
        None
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            let mut downloads = Vec::new();

            let urls = vec![
                ("https://storage.googleapis.com/flutter_infra_release/releases/releases_linux.json", Os::Linux),
                ("https://storage.googleapis.com/flutter_infra_release/releases/releases_macos.json", Os::Mac),
                ("https://storage.googleapis.com/flutter_infra_release/releases/releases_windows.json", Os::Windows),
            ];

            for (url, os) in urls {
                match reqwest::get(url).await {
                    Ok(response) => {
                        if let Ok(text) = response.text().await {
                            if let Ok(releases) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(releases_array) = releases["releases"].as_array() {
                                    for release in releases_array {
                                        if let (Some(version), Some(archive_url)) = (
                                            release["version"].as_str(),
                                            release["archive"].as_str(),
                                        ) {
                                            let mut tags = HashSet::new();

                                            if version.contains("beta") || version.contains("alpha")
                                            {
                                                tags.insert("beta".to_string());
                                            }

                                            if let Some(channel) = release["channel"].as_str() {
                                                if channel != "stable" {
                                                    tags.insert("beta".to_string());
                                                }
                                            }

                                            let absolute_url = if archive_url.starts_with("http") {
                                                archive_url.to_string()
                                            } else {
                                                format!("https://storage.googleapis.com/flutter_infra_release/releases/{}", archive_url)
                                            };

                                            let arch = if let Some(dart_sdk_arch) =
                                                release["dart_sdk_arch"].as_str()
                                            {
                                                match dart_sdk_arch {
                                                    "arm64" => Arch::Arm64,
                                                    "x64" => Arch::X86_64,
                                                    _ => Arch::X86_64,
                                                }
                                            } else {
                                                Arch::X86_64
                                            };

                                            downloads.push(Download {
                                                version: GgVersion::new(version),
                                                tags,
                                                download_url: absolute_url,
                                                os: Some(os),
                                                arch: Some(arch),
                                                variant: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Eh not sure
                    }
                }
            }

            downloads
        })
    }

    fn get_bins(&self, _input: &AppInput) -> Vec<BinPattern> {
        vec![
            BinPattern::Exact("bin/flutter".to_string()),
            BinPattern::Exact("bin/dart".to_string()),
        ]
    }

    fn get_name(&self) -> &str {
        "flutter"
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        vec!["beta".to_string()].into_iter().collect()
    }

    fn get_env(&self, app_path: &AppPath) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert(
            "FLUTTER_ROOT".to_string(),
            app_path.install_dir.to_string_lossy().to_string(),
        );

        let current_path = std::env::var("PATH").unwrap_or_default();
        let flutter_bin_path = app_path
            .install_dir
            .join("bin")
            .to_string_lossy()
            .to_string();
        let new_path = if current_path.is_empty() {
            flutter_bin_path
        } else {
            format!("{}:{}", flutter_bin_path, current_path)
        };
        env.insert("PATH".to_string(), new_path);

        env
    }

    fn post_download(&self, download_file_path: String) -> bool {
        let output = std::process::Command::new("tar")
            .arg("-xf")
            .arg(&download_file_path)
            .arg("-C")
            .arg(std::path::Path::new(&download_file_path).parent().unwrap())
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}
