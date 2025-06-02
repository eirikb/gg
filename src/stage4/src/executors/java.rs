use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::pin::Pin;

use semver::VersionReq;
use serde::Deserialize;

use crate::executor::{AppInput, AppPath, BinPattern, Download, ExecutorCmd};
use crate::executors::gradle_properties::GradleAndWrapperProperties;
use crate::executors::java_distributions::JavaDistributions;
use crate::target::Os;
use crate::Executor;

#[derive(Debug, Deserialize)]
struct SdkmanRc {
    java: Option<String>,
}

pub struct Java {
    pub executor_cmd: ExecutorCmd,
}

fn get_jdk_version() -> Option<String> {
    get_jdk_version_from_path(".")
}

fn get_jdk_version_from_path(base_path: &str) -> Option<String> {
    use std::path::Path;

    let java_version_path = Path::new(base_path).join(".java-version");
    if let Ok(content) = fs::read_to_string(&java_version_path) {
        let version = content.trim();
        if !version.is_empty() {
            return Some(version.to_string());
        }
    }

    let sdkmanrc_path = Path::new(base_path).join(".sdkmanrc");
    if let Ok(content) = fs::read_to_string(&sdkmanrc_path) {
        if let Ok(sdkmanrc) = serde_java_properties::from_str::<SdkmanRc>(&content) {
            if let Some(java_version) = sdkmanrc.java {
                return Some(java_version);
            }
        }
    }

    GradleAndWrapperProperties::new().get_jdk_version()
}

impl Java {
    fn get_distribution(&self) -> crate::executors::java_distributions::DistributionConfig {
        if let Some(ref dist_name) = self.executor_cmd.distribution {
            JavaDistributions::get_by_name(dist_name)
                .unwrap_or_else(|| JavaDistributions::get_default())
        } else {
            JavaDistributions::get_default()
        }
    }
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
            let distribution = self.get_distribution();
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
        let distribution = self.get_distribution();
        distribution
            .default_tags
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn create_isolated_test_dir() -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("java_test_{}", timestamp));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_get_jdk_version_from_java_version_file() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.join(".java-version");

        fs::write(&java_version_path, "17.0.1").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, Some("17.0.1".to_string()));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_get_jdk_version_from_java_version_file_with_whitespace() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.join(".java-version");

        fs::write(&java_version_path, "  21.0.2  \n").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, Some("21.0.2".to_string()));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_get_jdk_version_from_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let sdkmanrc_path = temp_dir.join(".sdkmanrc");

        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, Some("11.0.16-zulu".to_string()));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_get_jdk_version_priority() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.join(".java-version");
        let sdkmanrc_path = temp_dir.join(".sdkmanrc");

        fs::write(&java_version_path, "17.0.1").unwrap();
        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, Some("17.0.1".to_string()));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_get_jdk_version_empty_java_version_falls_back_to_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.join(".java-version");
        let sdkmanrc_path = temp_dir.join(".sdkmanrc");

        fs::write(&java_version_path, "").unwrap();
        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, Some("11.0.16-zulu".to_string()));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_get_jdk_version_invalid_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let sdkmanrc_path = temp_dir.join(".sdkmanrc");

        fs::write(&sdkmanrc_path, "invalid content").unwrap();

        let version = get_jdk_version_from_path(temp_dir.to_str().unwrap());
        assert_eq!(version, None);

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
