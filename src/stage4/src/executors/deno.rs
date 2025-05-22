use std::future::Future;
use std::pin::Pin;

use crate::executor::{AppInput, Download, Executor, ExecutorCmd};
use crate::executors::github::GitHub;

pub struct Deno {
    github_executor: GitHub,
}

impl Deno {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        let github_executor = GitHub::new_with_config(
            executor_cmd,
            "denoland".to_string(),
            "deno".to_string(),
            Some(vec![]),
            Some(vec!["deno".to_string(), "deno.exe".to_string()]),
        );

        Self { github_executor }
    }
}

impl Executor for Deno {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        self.github_executor.get_executor_cmd()
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        self.github_executor.get_download_urls(input)
    }

    fn get_bins(&self, input: &AppInput) -> Vec<String> {
        self.github_executor.get_bins(input)
    }

    fn get_name(&self) -> &str {
        "deno"
    }

    fn get_deps<'a>(&'a self) -> Pin<Box<dyn Future<Output = Vec<&'a str>> + 'a>> {
        self.github_executor.get_deps()
    }
}
