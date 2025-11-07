use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use semver::VersionReq;

use crate::executor::{
    find_jar_file, AppInput, AppPath, BinPattern, Download, ExecutorCmd, ExecutorDep,
};
use crate::target::{Arch, Os, Variant};
use crate::Executor;

pub struct CustomCommand {
    pub executor_cmd: ExecutorCmd,
}

impl CustomCommand {
    fn is_jar_url(url: &str) -> bool {
        (url.starts_with("http://") || url.starts_with("https://")) && url.ends_with(".jar")
    }
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
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            if let Some(first_arg) = input.app_args.first() {
                if Self::is_jar_url(first_arg) {
                    return vec![Download {
                        download_url: first_arg.clone(),
                        version: None,
                        os: Some(Os::Any),
                        arch: Some(Arch::Any),
                        variant: Some(Variant::Any),
                        tags: HashSet::new(),
                    }];
                }
            }
            vec![]
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        if let Some(first_arg) = input.app_args.first() {
            if Self::is_jar_url(first_arg) {
                return vec![BinPattern::Exact("java".to_string())];
            }
        }
        vec![BinPattern::Exact(input.app_args[0].as_str().to_string())]
    }

    fn get_name(&self) -> &str {
        "custom_command"
    }

    fn get_deps<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            if let Some(first_arg) = input.app_args.first() {
                if Self::is_jar_url(first_arg) {
                    return vec![ExecutorDep::new("java".to_string(), None)];
                }
            }
            vec![]
        })
    }

    fn customize_args(&self, input: &AppInput, app_path: &AppPath) -> Vec<String> {
        if let Some(jar_name) = find_jar_file(app_path) {
            if let Some(jar_path) = app_path.install_dir.join(&jar_name).to_str() {
                let mut args = vec!["-jar".to_string(), jar_path.to_string()];
                args.extend(input.app_args.iter().skip(1).cloned());
                return args;
            }
        }
        input.app_args.clone().into_iter().skip(1).collect()
    }

    fn custom_prep(&self, input: &AppInput) -> Option<AppPath> {
        if let Some(first_arg) = input.app_args.first() {
            if Self::is_jar_url(first_arg) {
                return None;
            }
        }
        Some(AppPath {
            install_dir: PathBuf::new(),
        })
    }
}
