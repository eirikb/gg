use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::pin::Pin;

use log::info;
use package_json::PackageJsonManager;
use regex::Regex;
use semver::{Version, VersionReq};
use serde::Deserialize;
use serde::Serialize;

use crate::executor::{AppInput, Download, Executor, ExecutorCmd};
use crate::target::{Arch, Os, Target, Variant};

pub struct Deno {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Deno {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(&self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move {
            let mut downloads: Vec<Download> = vec!();
            let octocrab = octocrab::instance();
            let mut page: u32 = 1;
            loop {
                let mut releases = octocrab.repos("denoland", "deno")
                    .releases().list().page(page).per_page(100).send().await.unwrap();
                for release in releases.items {
                    for asset in release.assets {
                        
                    }
                    downloads.push(Download::new(
                        "".to_string(), release.tag_name.as_str(), None,
                    ))
                }
                if releases.next == None {
                    break;
                }
                page += 1;
            }
            downloads
        })
    }

    fn get_bin(&self, input: &AppInput) -> Vec<&str> {
        vec!(match &input.target.os {
            Os::Windows => "deno.exe",
            _ => "deno"
        })
    }

    fn get_name(&self) -> &str {
        "deno"
    }
}
