use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use semver::VersionReq;

use crate::executor::{AppInput, AppPath, BinPattern, Download, ExecutorCmd};
use crate::executors::gradle_properties::GradleAndWrapperProperties;
use crate::executors::java_distributions::JavaDistributions;
use crate::target::Os;
use crate::Executor;


pub struct Java {
    pub executor_cmd: ExecutorCmd,
}

fn get_jdk_version() -> Option<String> {
    GradleAndWrapperProperties::new().get_jdk_version()
}

impl Executor for Java {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(jdk_version) = get_jdk_version() {
            if let Ok(version) = VersionReq::parse(jdk_version.as_str()) {
                return Some(version);
            }
        }

        None
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move { 
            let distribution = if let Some(ref dist_name) = self.executor_cmd.distribution {
                JavaDistributions::get_by_name(dist_name)
                    .unwrap_or_else(|| JavaDistributions::get_default())
            } else {
                JavaDistributions::get_default()
            };
            
            (distribution.handler)(&input.target).await
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => "java.exe",
                _ => "java",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "java"
    }

    fn get_default_include_tags(&self) -> HashSet<String> {
        let distribution = if let Some(ref dist_name) = self.executor_cmd.distribution {
            JavaDistributions::get_by_name(dist_name)
                .unwrap_or_else(|| JavaDistributions::get_default())
        } else {
            JavaDistributions::get_default()
        };
        
        distribution.default_tags
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_env(&self, app_path: &AppPath) -> HashMap<String, String> {
        [(
            String::from("JAVA_HOME"),
            app_path.install_dir.to_str().unwrap().to_string(),
        )]
        .iter()
        .cloned()
        .collect()
    }
}
