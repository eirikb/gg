use std::future::Future;
use std::pin::Pin;
use which::{which};

use semver::VersionReq;

use crate::{Executor};
use crate::executor::{AppInput, AppPath, Download, ExecutorCmd};

pub struct CustomCommand {
    executor_cmd: ExecutorCmd,
}

impl Executor for CustomCommand {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        None
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move { vec!() })
    }

    // TODO:
    fn get_bin(&self, _input: &AppInput) -> &str {
        self.executor_cmd.cmd.as_str()
    }

    fn get_name(&self) -> &str {
        "custom_command"
    }

    fn custom_prep(&self) -> Option<AppPath> {
        let cmd = self.executor_cmd.cmd.as_str();
        let bin = which(cmd.clone());
        if let Ok(bin) = bin {
            Some(AppPath {
                app: bin.clone(),
                bin,
            })
        } else {
            println!("Custom command {} not found", cmd);
            return None;
        }
    }
}
