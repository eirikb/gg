use std::fs;
use std::future::Future;
use std::pin::Pin;

use semver::VersionReq;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep};
use crate::executors::github::GitHub;

pub struct Bld {
    github: GitHub,
}

impl Bld {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        let github = GitHub::new_with_config(
            executor_cmd,
            "rife2".to_string(),
            "bld".to_string(),
            None,
            Some(vec!["bld".to_string(), "bld.bat".to_string()]),
        );

        Self { github }
    }

    fn get_bld_version() -> Option<String> {
        let properties_path = "lib/bld/bld-wrapper.properties";
        if let Ok(content) = fs::read_to_string(properties_path) {
            for line in content.lines() {
                if line.starts_with("bld.version=") {
                    return Some(line.trim_start_matches("bld.version=").to_string());
                }
            }
        }
        None
    }
}

impl Executor for Bld {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        self.github.get_executor_cmd()
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(version) = Self::get_bld_version() {
            if let Ok(version_req) = VersionReq::parse(&version) {
                return Some(version_req);
            }
        }
        None
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        self.github.get_download_urls(input)
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        self.github.get_bins(input)
    }

    fn get_name(&self) -> &str {
        self.github.get_name()
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            vec![ExecutorDep {
                name: "java".to_string(),
                version: None,
            }]
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_bld_version() {
        let content = "bld.version=2.2.1";
        let lines: Vec<&str> = content.lines().collect();
        let mut version = None;
        for line in lines {
            if line.starts_with("bld.version=") {
                version = Some(line.trim_start_matches("bld.version=").to_string());
            }
        }
        assert_eq!(version, Some("2.2.1".to_string()));
    }

    #[test]
    fn test_parse_bld_version_with_spaces() {
        let content = "bld.version = 2.2.1";
        let lines: Vec<&str> = content.lines().collect();
        let mut version = None;
        for line in lines {
            if line.trim().starts_with("bld.version") && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    version = Some(parts[1].trim().to_string());
                }
            }
        }
        assert_eq!(version, Some("2.2.1".to_string()));
    }
}
