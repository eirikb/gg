use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use semver::VersionReq;

use crate::executor::{AppInput, AppPath, BinPattern, Download, ExecutorCmd};
use crate::Executor;

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

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move { vec![] })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            input.app_args[0].as_str().to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "custom_command"
    }

    fn customize_args(&self, input: &AppInput, _app_path: &AppPath) -> Vec<String> {
        input.app_args.clone().into_iter().skip(1).collect()
    }

    fn custom_prep(&self, _input: &AppInput) -> Option<AppPath> {
        Some(AppPath {
            install_dir: PathBuf::new(),
        })
    }
}
