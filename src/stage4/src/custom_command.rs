use std::future::Future;
use std::pin::Pin;

use semver::VersionReq;
use which::which;

use crate::Executor;
use crate::executor::{AppInput, AppPath, Download, ExecutorCmd};

pub struct CustomCommand {
    pub executor_cmd: ExecutorCmd,
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

    fn get_bin(&self, _input: &AppInput) -> Vec<&str> {
        vec!(self.executor_cmd.cmd.as_str())
    }

    fn get_name(&self) -> &str {
        "custom_command"
    }

    fn customize_args(&self, input: &AppInput, _app_path: &AppPath) -> Vec<String> {
        input.no_clap.app_args.clone().into_iter().skip(1).collect()
    }

    fn custom_prep(&self, input: &AppInput) -> Option<AppPath> {
        let cmd = input.no_clap.app_args.clone().into_iter().next().unwrap_or("".to_string());
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
