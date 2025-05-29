use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use log::{debug, info};
use scraper::{Html, Selector};
use semver::VersionReq;
use sha256::try_digest;

use crate::executor::{java_deps, AppInput, BinPattern, Download, ExecutorCmd, ExecutorDep};
use crate::executors::gradle_properties::GradleAndWrapperProperties;
use crate::target::Variant;
use crate::{target, Executor};

pub struct Gradle {
    pub executor_cmd: ExecutorCmd,
    props: GradleAndWrapperProperties,
}

impl Gradle {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        Self {
            executor_cmd,
            props: GradleAndWrapperProperties::new(),
        }
    }
}

impl Executor for Gradle {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(version) = self.props.get_version_from_distribution_url() {
            return VersionReq::parse(version.as_str()).ok();
        }
        None
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            if let Some(distribution_url) = self.props.get_distribution_url() {
                if let Some(version) = self.props.get_version_from_distribution_url() {
                    return vec![Download::new(
                        distribution_url,
                        version.as_str(),
                        Some(Variant::Any),
                    )];
                }
            }

            let body = reqwest::get("https://gradle.org/releases")
                .await
                .expect("Unable to connect to services.gradle.org")
                .text()
                .await
                .expect("Unable to download gradle list of versions");

            let document = Html::parse_document(body.as_str());
            document
                .select(&Selector::parse("a[name]").unwrap())
                .map(|link| {
                    let version = link.value().attr("name").unwrap_or("").to_string();
                    Download::new(
                        format!(
                            "https://services.gradle.org/distributions/gradle-{version}-bin.zip"
                        ),
                        version.as_str(),
                        Some(Variant::Any),
                    )
                })
                .collect()
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                target::Os::Windows => "gradle.bat",
                _ => "gradle",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "gradle"
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        java_deps()
    }

    fn post_download(&self, download_file_path: String) -> bool {
        if let Some(checksum) = self.props.get_distribution_sha256sum() {
            info!("Checksum found for {}: {}", &download_file_path, checksum);
            debug!("Calculating checksum for {}", &download_file_path);
            let input = Path::new(download_file_path.as_str());
            let val = try_digest(input).unwrap();
            info!("Calculated checksum: {}", val);
            return checksum == val;
        }
        debug!("No checksum found in gradle properties (skipping check)");
        true
    }
}
