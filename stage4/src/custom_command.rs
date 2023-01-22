use std::future::Future;
use std::pin::Pin;
use which::{which};

use semver::VersionReq;

use crate::{Executor};
use crate::executor::{AppInput, AppPath, Download};

pub struct CustomCommand {
    pub cmd: String,
}

impl Executor for CustomCommand {
    fn get_version_req(&self) -> Option<VersionReq> {
        None
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move { vec!() })
    }

    fn get_bin(&self, _input: &AppInput) -> &str {
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
