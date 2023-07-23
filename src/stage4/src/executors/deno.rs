use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::executor::{AppInput, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os, Variant};
use crate::target::Os::Windows;

pub struct Deno {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Deno {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(&self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move {
            let mut downloads: Vec<Download> = vec!();
            let octocrab = octocrab::Octocrab::builder().base_uri("https://ggcmddenogithubreleases.azureedge.net/").unwrap().build().unwrap();
            let mut page: u32 = 1;
            loop {
                let releases = octocrab.repos("denoland", "deno")
                    .releases().list().page(page).per_page(100).send().await.unwrap();
                for release in releases.items {
                    for asset in release.assets {
                        let os = if asset.name.contains("windows") {
                            Some(Windows)
                        } else if asset.name.contains("linux") {
                            Some(Os::Linux)
                        } else if asset.name.contains("apple") {
                            Some(Os::Mac)
                        } else {
                            None
                        };
                        let arch = if asset.name.contains("x86_64") {
                            Some(Arch::X86_64)
                        } else {
                            None
                        };
                        if os.is_some() && arch.is_some() {
                            downloads.push(Download {
                                download_url: asset.browser_download_url.to_string(),
                                version: GgVersion::new(release.tag_name.as_str()),
                                os,
                                arch,
                                tags: HashSet::new(),
                                variant: Some(Variant::Any),
                            });
                        }
                    }
                }
                if releases.next == None {
                    break;
                }
                page += 1;
            }
            downloads
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<String> {
        vec!(match &input.target.os {
            Windows => "deno.exe",
            _ => "deno"
        }.to_string())
    }

    fn get_name(&self) -> &str {
        "deno"
    }
}
