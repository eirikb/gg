use std::collections::HashSet;
use std::fs::rename;
use std::future::Future;
use std::pin::Pin;

use crate::bloody_maven::get_download_urls_from_maven;
use crate::executor::{
    java_deps, AppInput, AppPath, BinPattern, Download, ExecutorCmd, ExecutorDep,
};
use crate::Executor;

pub struct OpenAPIGenerator {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for OpenAPIGenerator {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        get_download_urls_from_maven("openapitools", "openapi-generator-cli")
    }

    fn get_bins(&self, _input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact("java".to_string())]
    }

    fn get_name(&self) -> &str {
        "openapi"
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        java_deps()
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        vec!["beta"].into_iter().map(|s| s.to_string()).collect()
    }

    fn customize_args(&self, input: &AppInput, app_path: &AppPath) -> Vec<String> {
        let jar = "openapi-generator-cli.jar";
        if let Some(path) = app_path.install_dir.join(jar).to_str() {
            let args = vec!["-jar".to_string(), path.to_string()];
            args.iter()
                .cloned()
                .chain(input.app_args.iter().cloned())
                .collect()
        } else {
            vec![]
        }
    }

    fn post_prep(&self, cache_path: &str) {
        let pattern = format!("{}/*openapi-generator-cli*.jar", cache_path);
        if let Ok(paths) = glob::glob(&pattern) {
            for path in paths.flatten() {
                rename(&path, format!("{}/openapi-generator-cli.jar", cache_path)).unwrap();
            }
        }
    }
}
