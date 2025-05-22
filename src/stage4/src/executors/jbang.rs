use std::future::Future;
use std::pin::Pin;

use crate::executor::{AppInput, Download, Executor, ExecutorCmd};
use crate::executors::github::GitHub;

pub struct JBang {
    github_executor: GitHub,
}

impl JBang {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        let github_executor = GitHub::new_with_config(
            executor_cmd,
            "jbangdev".to_string(),
            "jbang".to_string(),
            Some(vec!["java".to_string()]),
            Some(vec!["jbang".to_string(), "jbang.exe".to_string()]),
        );

        Self { github_executor }
    }
}

impl Executor for JBang {
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
        "jbang"
    }

    fn get_deps<'a>(&'a self) -> Pin<Box<dyn Future<Output = Vec<&'a str>> + 'a>> {
        self.github_executor.get_deps()
    }
}
