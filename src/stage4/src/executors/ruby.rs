use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use log::{debug, info};

use crate::executor::{AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::github_utils::{create_github_client, detect_arch_from_name};
use crate::target::{Arch, Os, Variant};

pub struct Ruby {
    pub executor_cmd: ExecutorCmd,
}

fn is_ruby_binary(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    name_lower.contains("rubyinstaller")
        && (name_lower.ends_with(".7z") || name_lower.ends_with(".exe"))
}

impl Ruby {
    fn install_gem(&self, gem_name: &str, cache_path: &str) {
        use std::process::Command;

        info!("Installing gem {} for Ruby at {}", gem_name, cache_path);

        let gem_home = std::path::Path::new(cache_path).join("gem_home");
        std::fs::create_dir_all(&gem_home).ok();

        let ruby_bin_dir = std::path::Path::new(cache_path).join("bin");
        let gem_bin = ruby_bin_dir.join("gem");

        if gem_bin.exists() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let ruby_bin_path = ruby_bin_dir.to_string_lossy();
            let new_path = if current_path.is_empty() {
                ruby_bin_path.to_string()
            } else {
                format!("{}:{}", ruby_bin_path, current_path)
            };

            let output = Command::new(&gem_bin)
                .args(["install", gem_name, "--no-document", "--install-dir"])
                .arg(&gem_home)
                .env("GEM_HOME", &gem_home)
                .env("GEM_PATH", &gem_home)
                .env("PATH", &new_path)
                .output();

            match output {
                Ok(result) => {
                    if result.status.success() {
                        info!("Successfully installed gem {}", gem_name);
                    } else {
                        debug!(
                            "Gem install failed: {}",
                            String::from_utf8_lossy(&result.stderr)
                        );
                    }
                }
                Err(e) => {
                    debug!("Failed to run gem install: {}", e);
                }
            }
        } else {
            debug!("Ruby gem command not found at {:?}", gem_bin);
        }
    }
}

impl Executor for Ruby {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            match &input.target.os {
                Os::Windows => get_windows_ruby_urls().await,
                Os::Linux | Os::Mac => get_truffleruby_urls(&input.target.os).await,
                _ => vec![],
            }
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        match &input.target.os {
            Os::Windows => vec![
                BinPattern::Exact("bin/ruby.exe".to_string()),
                BinPattern::Exact("bin/gem".to_string()),
                BinPattern::Exact("bin/gem.cmd".to_string()),
                BinPattern::Exact("bin/bundle".to_string()),
                BinPattern::Exact("bin/irb".to_string()),
                BinPattern::Exact("ruby.exe".to_string()),
                BinPattern::Exact("gem_home/bin/gem".to_string()),
                BinPattern::Exact("gem_home/bin/bundle".to_string()),
                BinPattern::Exact("gem_home/bin/irb".to_string()),
            ],
            _ => {
                let mut bins = vec![
                    BinPattern::Exact("bin/ruby".to_string()),
                    BinPattern::Exact("bin/gem".to_string()),
                    BinPattern::Exact("bin/bundle".to_string()),
                    BinPattern::Exact("bin/irb".to_string()),
                    BinPattern::Exact("gem_home/bin/gem".to_string()),
                    BinPattern::Exact("gem_home/bin/bundle".to_string()),
                    BinPattern::Exact("gem_home/bin/irb".to_string()),
                ];

                if let Some(gems) = &self.executor_cmd.gems {
                    for gem_name in gems {
                        bins.push(BinPattern::Exact(format!("gem_home/bin/{}", gem_name)));
                    }
                }

                bins
            }
        }
    }

    fn get_name(&self) -> &str {
        "ruby"
    }

    fn get_env(&self, app_path: &AppPath) -> HashMap<String, String> {
        let mut env = HashMap::new();

        let gem_home = app_path.install_dir.join("gem_home");
        let gem_home_str = gem_home.to_string_lossy().to_string();

        env.insert("GEM_HOME".to_string(), gem_home_str.clone());
        env.insert("GEM_PATH".to_string(), gem_home_str);

        env
    }

    fn post_prep(&self, cache_path: &str) {
        #[cfg(unix)]
        {
            let ruby_bin_dir = std::path::Path::new(cache_path).join("bin");
            let rake_path = ruby_bin_dir.join("rake");
            let trufflerake_path = ruby_bin_dir.join("trufflerake");

            if rake_path.exists() && !trufflerake_path.exists() {
                std::os::unix::fs::symlink(&rake_path, &trufflerake_path).ok();
            }
        }

        if let Some(gems) = &self.executor_cmd.gems {
            for gem_name in gems {
                self.install_gem(gem_name, cache_path);
            }
        }

        let gem_bin_dir = std::path::Path::new(cache_path)
            .join("gem_home")
            .join("bin");
        if gem_bin_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&gem_bin_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().unwrap().is_file() {
                        let exe_path = entry.path();
                        let ruby_bin = std::path::Path::new(cache_path).join("bin").join("ruby");

                        if let Ok(content) = std::fs::read_to_string(&exe_path) {
                            if content.starts_with("#!/usr/bin/ruby") {
                                let new_content = content.replace(
                                    "#!/usr/bin/ruby",
                                    &format!("#!{}", ruby_bin.to_string_lossy()),
                                );
                                let _ = std::fs::write(&exe_path, new_content);
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn get_truffleruby_urls(os: &Os) -> Vec<Download> {
    let mut downloads = vec![];

    let octocrab = create_github_client().unwrap();

    if let Ok(releases) = octocrab
        .repos("ruby", "ruby-builder")
        .releases()
        .list()
        .per_page(50)
        .send()
        .await
    {
        for release in releases.items {
            for asset in release.assets {
                let name_lower = asset.name.to_lowercase();

                let matches_os = match os {
                    Os::Linux => name_lower.contains("ubuntu"),
                    Os::Mac => name_lower.contains("macos"),
                    Os::Windows => name_lower.contains("windows"),
                    _ => false,
                };

                if !matches_os || !name_lower.ends_with(".tar.gz") {
                    continue;
                }

                if !name_lower.starts_with("truffleruby-") {
                    continue;
                }

                let arch = if name_lower.contains("x86_64") {
                    Some(Arch::X86_64)
                } else if name_lower.contains("arm64") || name_lower.contains("aarch64") {
                    Some(Arch::Arm64)
                } else {
                    Some(Arch::Any)
                };

                debug!(
                    "TruffleRuby asset: {} -> OS: {:?}, Arch: {:?}",
                    asset.name, os, arch
                );

                if let Some(version) = extract_truffleruby_version(&asset.name) {
                    downloads.push(Download {
                        download_url: asset.browser_download_url.to_string(),
                        version: GgVersion::new(&version),
                        os: Some(*os),
                        arch,
                        tags: HashSet::new(),
                        variant: Some(Variant::Any),
                    });
                }
            }
        }
    }

    debug!("Total TruffleRuby downloads found: {}", downloads.len());
    downloads
}

async fn get_windows_ruby_urls() -> Vec<Download> {
    let mut downloads = vec![];

    let octocrab = create_github_client().unwrap();

    if let Ok(releases) = octocrab
        .repos("oneclick", "rubyinstaller2")
        .releases()
        .list()
        .per_page(50)
        .send()
        .await
    {
        for release in releases.items {
            for asset in release.assets {
                if !is_ruby_binary(&asset.name) {
                    continue;
                }

                let arch = detect_arch_from_name(&asset.name);

                debug!("RubyInstaller asset: {} -> Arch: {:?}", asset.name, arch);

                if let Some(arch) = arch {
                    downloads.push(Download {
                        download_url: asset.browser_download_url.to_string(),
                        version: GgVersion::new(&release.tag_name.replace("RubyInstaller-", "")),
                        os: Some(Os::Windows),
                        arch: Some(arch),
                        tags: HashSet::new(),
                        variant: Some(Variant::Any),
                    });
                }
            }
        }
    }

    debug!("Total Windows Ruby downloads found: {}", downloads.len());
    downloads
}

fn extract_truffleruby_version(filename: &str) -> Option<String> {
    if let Some(start) = filename.find("truffleruby-") {
        let after_prefix = &filename[start + 12..];
        if let Some(end) = after_prefix.find('-') {
            let version = &after_prefix[..end];
            return Some(version.to_string());
        }
    }
    None
}
