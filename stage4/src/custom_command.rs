use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use which::{which, WhichConfig};

use java_properties::read;
use log::debug;
use regex::Regex;
use scraper::{Html, Selector};
use semver::VersionReq;

use crate::{Executor, Java};
use crate::cmd_to_executor::cmd_to_executor;
use crate::executor::{AppInput, AppPath, Download, prep};

use super::target;

pub struct CustomCommand {
    pub cmd: String,
}

impl Executor for CustomCommand {
    fn get_version_req(&self) -> Option<VersionReq> {
        None
    }

    fn get_download_urls<'a>(&'a self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move { vec!() })
    }

    fn get_bin(&self, input: &AppInput) -> &str {
        self.cmd.as_str()
    }

    fn get_name(&self) -> &str {
        "custom_command"
    }

    fn custom_prep(&self) -> Option<AppPath> {
        let bin = which(self.cmd.clone());
        if let Ok(bin) = bin {
            Some(AppPath {
                app: bin.clone(),
                bin,
            })
        } else {
            println!("Custom command {} not found", self.cmd);
            return None;
        }
    }
}
