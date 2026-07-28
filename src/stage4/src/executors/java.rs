use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::pin::Pin;

use semver::VersionReq;
use serde::Deserialize;

use crate::executor::{AppInput, AppPath, BinPattern, Download, ExecutorCmd};
use crate::executors::gradle_properties::GradleAndWrapperProperties;
use crate::executors::java_distributions::{JavaDistributions, FALLBACK_DISTRIBUTION};
use crate::target::Os;
use crate::Executor;

#[derive(Debug, Deserialize)]
struct SdkmanRc {
    java: Option<String>,
}

pub struct Java {
    pub executor_cmd: ExecutorCmd,
}

fn has_matching_version(downloads: &[Download], version_req: &VersionReq) -> bool {
    downloads.iter().any(|download| {
        download
            .version
            .as_ref()
            .is_some_and(|version| version_req.matches(&version.to_version()))
    })
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
                .unwrap_or_else(JavaDistributions::get_default)
        } else {
            JavaDistributions::get_default()
        }
    }

    /// `java@-temurin` picks a distribution, `java@-jdk+jre` drops a tag, and both are
    /// `@-word` - so a name that isn't a registered distribution meant the tag removal.
    fn dropped_tag(&self) -> Option<&str> {
        let name = self.executor_cmd.distribution.as_deref()?;
        match JavaDistributions::get_by_name(name) {
            Some(_) => None,
            None => Some(name),
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
            let downloads = (distribution.handler)(&input.target).await;

            // An explicit -azul/-tem is the user's call, so never second-guess it.
            if self.executor_cmd.distribution.is_some() && self.dropped_tag().is_none() {
                return downloads;
            }

            // Tags are applied after this, and Temurin has none of the +fx/+headless/+sts
            // that only Azul publishes - so a version-only check says yes, then matches
            // nothing.
            let matches = (self as &dyn Executor).get_url_matches(&downloads, input);

            // Same precedence as executor.rs, and get_url_matches only knows
            // executor_cmd.version - so .sdkmanrc and gradle.properties land here.
            let version_req = match &self.executor_cmd.version {
                Some(version_req) => Some(version_req.to_version_req()),
                None => self.get_version_req(),
            };

            let needs_fallback = match &version_req {
                Some(version_req) => !has_matching_version(&matches, version_req),
                None => matches.is_empty(),
            };

            if needs_fallback {
                if let Some(fallback) = JavaDistributions::get_by_name(FALLBACK_DISTRIBUTION) {
                    let fallback_downloads = (fallback.handler)(&input.target).await;
                    // Empty means Azul failed too - keep what we have instead
                    if !fallback_downloads.is_empty() {
                        return fallback_downloads;
                    }
                }
            }

            downloads
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
        let mut tags: HashSet<String> = distribution
            .default_tags
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        // Defaults are required tags, so a dropped one has to leave here too, or
        // java@-jdk+jre wants jre and jdk at once and matches nothing
        if let Some(tag) = self.dropped_tag() {
            tags.remove(tag);
        }
        for tag in &self.executor_cmd.exclude_tags {
            tags.remove(tag.as_str());
        }

        tags
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        match self.dropped_tag() {
            Some(tag) => HashSet::from([tag.to_string()]),
            None => HashSet::new(),
        }
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
    use std::fs;
    use tempfile::TempDir;

    // A guaranteed-unique temp dir that auto-removes on drop. (The old
    // timestamp-named dirs could collide under parallel test runs, making
    // one test delete another's dir mid-flight.)
    fn create_isolated_test_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn downloads_for(versions: &[&str]) -> Vec<Download> {
        versions
            .iter()
            .map(|version| Download::new(format!("http://x/{version}"), version, None))
            .collect()
    }

    fn version_req(req: &str) -> VersionReq {
        crate::executor::GgVersionReq::new(req)
            .unwrap()
            .to_version_req()
    }

    fn java_with(distribution: Option<&str>, include: &[&str], exclude: &[&str]) -> Java {
        Java {
            executor_cmd: ExecutorCmd {
                cmd: "java".to_string(),
                version: None,
                distribution: distribution.map(String::from),
                include_tags: include.iter().map(|s| s.to_string()).collect(),
                exclude_tags: exclude.iter().map(|s| s.to_string()).collect(),
                gems: None,
            },
        }
    }

    #[test]
    fn test_real_distribution_is_not_treated_as_a_dropped_tag() {
        assert_eq!(java_with(Some("temurin"), &[], &[]).dropped_tag(), None);
    }

    #[test]
    fn test_unknown_distribution_is_a_dropped_tag() {
        // java@-jdk+jre parks "jdk" in the distribution slot - same syntax as java@-temurin
        assert_eq!(
            java_with(Some("jdk"), &["jre"], &[]).dropped_tag(),
            Some("jdk")
        );
    }

    #[test]
    fn test_dropped_tag_leaves_defaults_alone_for_a_real_distribution() {
        let tags = java_with(Some("temurin"), &[], &[]).get_default_include_tags();
        assert!(tags.contains("jdk"));
        assert!(java_with(Some("temurin"), &[], &[])
            .get_default_exclude_tags()
            .is_empty());
    }

    #[test]
    fn test_jre_request_drops_the_default_jdk_tag() {
        // Both halves matter, or java@-jdk+jre asks for jre while still requiring jdk
        let java = java_with(Some("jdk"), &["jre"], &[]);
        let include = java.get_default_include_tags();
        assert!(!include.contains("jdk"), "jdk should no longer be required");
        assert!(include.contains("ga"), "unrelated defaults should survive");
        assert!(java.get_default_exclude_tags().contains("jdk"));
    }

    #[test]
    fn test_explicit_exclude_tag_also_drops_the_default() {
        let include = java_with(None, &[], &["jdk"]).get_default_include_tags();
        assert!(!include.contains("jdk"));
        assert!(include.contains("ga"));
    }

    #[test]
    fn test_default_distribution_is_temurin() {
        assert_eq!(JavaDistributions::get_default().name, "temurin");
    }

    #[test]
    fn test_fallback_distribution_is_registered() {
        // Looked up by name at runtime, so a rename would just lose the fallback quietly
        assert!(JavaDistributions::get_by_name(FALLBACK_DISTRIBUTION).is_some());
    }

    #[test]
    fn test_has_matching_version_finds_requested_release() {
        let downloads = downloads_for(&["17.0.9", "21.0.1"]);
        assert!(has_matching_version(&downloads, &version_req("21")));
    }

    #[test]
    fn test_has_matching_version_misses_release_adoptium_lacks() {
        // java@14 404s on Adoptium, which is what sends us to Azul.
        let downloads = downloads_for(&["8.0.392", "17.0.9", "21.0.1"]);
        assert!(!has_matching_version(&downloads, &version_req("14")));
    }

    #[test]
    fn test_has_matching_version_on_empty_downloads() {
        // An unreachable Adoptium looks like this, not like an error.
        assert!(!has_matching_version(&[], &version_req("21")));
    }

    #[test]
    fn test_has_matching_version_ignores_versionless_downloads() {
        let downloads = vec![Download::new(
            "http://x/nightly".to_string(),
            "nightly",
            None,
        )];
        assert!(!has_matching_version(&downloads, &version_req("21")));
    }

    #[test]
    fn test_get_jdk_version_from_java_version_file() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.path().join(".java-version");

        fs::write(&java_version_path, "17.0.1").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, Some("17.0.1".to_string()));
    }

    #[test]
    fn test_get_jdk_version_from_java_version_file_with_whitespace() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.path().join(".java-version");

        fs::write(&java_version_path, "  21.0.2  \n").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, Some("21.0.2".to_string()));
    }

    #[test]
    fn test_get_jdk_version_from_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let sdkmanrc_path = temp_dir.path().join(".sdkmanrc");

        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, Some("11.0.16-zulu".to_string()));
    }

    #[test]
    fn test_get_jdk_version_priority() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.path().join(".java-version");
        let sdkmanrc_path = temp_dir.path().join(".sdkmanrc");

        fs::write(&java_version_path, "17.0.1").unwrap();
        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, Some("17.0.1".to_string()));
    }

    #[test]
    fn test_get_jdk_version_empty_java_version_falls_back_to_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let java_version_path = temp_dir.path().join(".java-version");
        let sdkmanrc_path = temp_dir.path().join(".sdkmanrc");

        fs::write(&java_version_path, "").unwrap();
        fs::write(&sdkmanrc_path, "java=11.0.16-zulu").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, Some("11.0.16-zulu".to_string()));
    }

    #[test]
    fn test_get_jdk_version_invalid_sdkmanrc() {
        let temp_dir = create_isolated_test_dir();
        let sdkmanrc_path = temp_dir.path().join(".sdkmanrc");

        fs::write(&sdkmanrc_path, "invalid content").unwrap();

        let version = get_jdk_version_from_path(temp_dir.path().to_str().unwrap());
        assert_eq!(version, None);
    }
}
