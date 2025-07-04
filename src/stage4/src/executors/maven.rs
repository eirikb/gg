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

fn get_version(link: &str) -> String {
    link.replace("apache-maven-", "")
        .replace("maven-", "")
        .replace("-bin.tar.gz", "")
        .replace(".tar.gz", "")
        .to_string()
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
            let url = "https://archive.apache.org/dist/maven/binaries/";
            let body = reqwest::get(url)
                .await
                .expect("Unable to connect to archive.apache.org")
                .text()
                .await
                .expect("Unable to download maven list of versions");

            let document = Html::parse_document(body.as_str());
            document
                .select(&Selector::parse("a").unwrap())
                .map(|a| a.text().next().unwrap_or("").trim())
                .filter(|link| link.contains("maven") && link.ends_with("tar.gz"))
                .map(|link| {
                    let mut tags = HashSet::new();
                    if link.contains("alpha") {
                        tags.insert("alpha".to_string());
                    }
                    if link.contains("beta") {
                        tags.insert("beta".to_string());
                    }
                    Download {
                        download_url: format!("{url}{link}"),
                        version: GgVersion::new(get_version(link).as_str()),
                        os: Some(Os::Any),
                        arch: Some(Arch::Any),
                        variant: Some(Variant::Any),
                        tags,
                    }
                })
                .collect()
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
}

