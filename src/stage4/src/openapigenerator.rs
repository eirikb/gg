use std::collections::HashSet;
use std::fs::{read_dir, rename};
use std::future::Future;
use std::pin::Pin;

use which::{Error, which_in_global};

use crate::bloody_maven::get_download_urls_from_maven;
use crate::Executor;
use crate::executor::{AppInput, AppPath, Download, ExecutorCmd};

pub struct OpenAPIGenerator {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for OpenAPIGenerator {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        return &self.executor_cmd;
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        get_download_urls_from_maven("openapitools", "openapi-generator-cli")
    }

    fn get_bin(&self, _input: &AppInput) -> Vec<&str> { vec!("openapi-generator-cli.jar") }

    fn get_name(&self) -> &str {
        "openapi"
    }

    fn get_deps(&self) -> Vec<&str> {
        vec!("java")
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        vec!["beta"].into_iter().map(|s| s.to_string()).collect()
    }

    fn get_custom_bin_path(&self, paths: &str) -> Option<String> {
        which_in_global("java", Some(paths)).and_then(|mut s| s.next().ok_or(Error::CannotFindBinaryPath)).map(|s| s.to_str().unwrap().to_string()).ok()
    }


    fn get_additional_args(&self, app_path: &AppPath) -> Vec<String> {
        if let Some(path) = app_path.bin.to_str().map(|s| s.to_string()) {
            vec!("-jar".to_string(), path)
        } else {
            vec!()
        }
    }

    fn post_prep(&self, cache_path: &str) {
        let entries = read_dir(&cache_path);
        if let Ok(entries) = entries {
            entries.for_each(|entry| {
                if let Ok(entry) = entry {
                    if let Some(path_str) = entry.path().to_str() {
                        if path_str.contains("openapi-generator-cli") && path_str.ends_with("jar") {
                            rename(entry.path(), cache_path.to_string() + "/openapi-generator-cli.jar").unwrap();
                        }
                    }
                }
            });
        }
    }
}
