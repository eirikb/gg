use std::collections::HashSet;
use std::fs::{read_dir, rename};
use std::future::Future;
use std::pin::Pin;

use crate::bloody_maven::get_download_urls_from_maven;
use crate::executor::{AppInput, AppPath, Download, ExecutorCmd};
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

    fn get_bins(&self, _input: &AppInput) -> Vec<String> {
        vec!["java".to_string()]
    }

    fn get_name(&self) -> &str {
        "openapi"
    }

    fn get_deps<'a>(&'a self) -> Pin<Box<dyn Future<Output = Vec<&'a str>> + 'a>> {
        Box::pin(async move { vec!["java"] })
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
                .chain(input.no_clap.app_args.iter().cloned())
                .collect()
        } else {
            vec![]
        }
    }

    fn post_prep(&self, cache_path: &str) {
        let entries = read_dir(&cache_path);
        if let Ok(entries) = entries {
            entries.for_each(|entry| {
                if let Ok(entry) = entry {
                    if let Some(path_str) = entry.path().to_str() {
                        if path_str.contains("openapi-generator-cli") && path_str.ends_with("jar") {
                            rename(
                                entry.path(),
                                cache_path.to_string() + "/openapi-generator-cli.jar",
                            )
                            .unwrap();
                        }
                    }
                }
            });
        }
    }
}
