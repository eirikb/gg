use std::fs;
use std::fs::{read_dir, rename};
use std::future::Future;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::pin::Pin;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os, Variant};

pub struct Rat {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Rat {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            let versions: Vec<String> =
                reqwest::get("https://ratbinsa.z1.web.core.windows.net/list.json")
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
            versions
                .into_iter()
                .map(|name| {
                    let url = format!("https://ratbinsa.z1.web.core.windows.net/{}", name);
                    let name = name.clone();
                    let parts = name.split("-");
                    let version = parts.clone().nth(1).unwrap_or("NA");
                    let os = match parts.clone().nth(2) {
                        Some("windows") => Some(Os::Windows),
                        Some("linux") => Some(Os::Linux),
                        Some("macos") => Some(Os::Mac),
                        _ => None,
                    };
                    Download {
                        version: GgVersion::new(version),
                        tags: Default::default(),
                        download_url: url,
                        arch: Some(Arch::X86_64),
                        os,
                        variant: Some(Variant::Any),
                    }
                })
                .collect()
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => "rat.exe",
                _ => "rat.bin",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "rat"
    }

    fn post_prep(&self, cache_path: &str) {
        let entries = read_dir(&cache_path);
        if let Ok(entries) = entries {
            entries.for_each(|entry| {
                if let Ok(entry) = entry {
                    if let Some(path_str) = entry.path().to_str() {
                        let to_path = if path_str.ends_with(".bin") {
                            Some(cache_path.to_string() + "/rat.bin")
                        } else if path_str.ends_with(".exe") {
                            Some(cache_path.to_string() + "/rat.exe")
                        } else {
                            None
                        };
                        if let Some(to_path) = to_path {
                            rename(entry.path(), &to_path).unwrap();
                            #[cfg(unix)]
                            {
                                let mut perms = fs::metadata(&to_path).unwrap().permissions();
                                perms.set_mode(0o755);
                                fs::set_permissions(to_path, perms).unwrap();
                            }
                        }
                    }
                }
            });
        }
    }
}
