use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use scraper::{Html, Selector};

use crate::executor::{
    java_deps, AppInput, BinPattern, Download, ExecutorCmd, ExecutorDep, GgVersion,
};
use crate::target::{Arch, Os, Variant};
use crate::Executor;

pub struct Maven {
    pub executor_cmd: ExecutorCmd,
}

fn get_tags(version: &str) -> HashSet<String> {
    let mut tags = HashSet::new();
    if version.contains("alpha") {
        tags.insert("alpha".to_string());
    }
    if version.contains("beta") {
        tags.insert("beta".to_string());
    }
    if version.contains("-rc-") {
        tags.insert("rc".to_string());
    }
    tags
}

async fn fetch_versions_from_directory(base_url: &str) -> Vec<Download> {
    let body = match reqwest::get(base_url).await {
        Ok(response) => match response.text().await {
            Ok(text) => text,
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };

    let document = Html::parse_document(&body);
    let selector = Selector::parse("a").unwrap();

    document
        .select(&selector)
        .filter_map(|a| {
            let href = a.value().attr("href")?;
            if !href.ends_with('/') || href == "../" {
                return None;
            }
            let version = href.trim_end_matches('/');
            if !version.chars().next()?.is_ascii_digit() {
                return None;
            }
            Some(version.to_string())
        })
        .map(|version| {
            let download_url = format!(
                "{}{}/binaries/apache-maven-{}-bin.tar.gz",
                base_url, version, version
            );
            Download {
                download_url,
                version: GgVersion::new(&version),
                os: Some(Os::Any),
                arch: Some(Arch::Any),
                variant: Some(Variant::Any),
                tags: get_tags(&version),
            }
        })
        .collect()
}

impl Executor for Maven {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            let mut downloads = Vec::new();

            let maven1_versions =
                fetch_versions_from_directory("https://archive.apache.org/dist/maven/maven-1/")
                    .await;
            downloads.extend(maven1_versions);

            let maven2_versions =
                fetch_versions_from_directory("https://archive.apache.org/dist/maven/maven-2/")
                    .await;
            downloads.extend(maven2_versions);

            let maven3_versions =
                fetch_versions_from_directory("https://archive.apache.org/dist/maven/maven-3/")
                    .await;
            downloads.extend(maven3_versions);

            let maven4_versions =
                fetch_versions_from_directory("https://archive.apache.org/dist/maven/maven-4/")
                    .await;
            downloads.extend(maven4_versions);

            downloads
        })
    }

    fn get_bins(&self, _input: &AppInput) -> Vec<BinPattern> {
        vec![
            BinPattern::Exact("mvn".to_string()),
            BinPattern::Exact("mvn.bat".to_string()),
            BinPattern::Exact("maven.bat".to_string()),
        ]
    }

    fn get_name(&self) -> &str {
        "maven"
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        java_deps()
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        vec!["alpha".to_string(), "beta".to_string(), "rc".to_string()]
            .into_iter()
            .collect()
    }
}
