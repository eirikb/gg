use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use scraper::{Html, Selector};
use semver::Version;

use crate::Executor;
use crate::executor::{AppInput, Download, ExecutorCmd};
use crate::target::{Arch, Os, Variant};

pub struct Maven {
    pub executor_cmd: ExecutorCmd,
}

fn get_version(link: &str) -> String {
    link.replace("apache-maven-", "").replace("maven-", "").replace("-bin.tar.gz", "").replace(".tar.gz", "").to_string()
}

impl Executor for Maven {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        return &self.executor_cmd;
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move {
            let url = "https://archive.apache.org/dist/maven/binaries/";
            let body = reqwest::get(url).await
                .expect("Unable to connect to archive.apache.org").text().await
                .expect("Unable to download maven list of versions");

            let document = Html::parse_document(body.as_str());
            document.select(&Selector::parse("a").unwrap())
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
                        version: Version::parse(get_version(link).as_str()).ok(),
                        os: Some(Os::Any),
                        arch: Some(Arch::Any),
                        variant: Some(Variant::Any),
                        tags,
                    }
                }).collect()
        })
    }

    fn get_bin(&self, input: &AppInput) -> Vec<&str> {
        match &input.target.os {
            Os::Windows => vec!("bin/mvn.cmd", "bin/mvn.bat", "maven.bat"),
            _ => vec!("bin/mvn", "bin/maven")
        }
    }

    fn get_name(&self) -> &str {
        "maven"
    }

    fn get_deps(&self) -> Vec<&str> {
        vec!("java")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hello_maven() {
        let mvn = Maven { executor_cmd: ExecutorCmd::dummy() };
        let app_input = AppInput::dummy();
        let urls = mvn.get_download_urls(&app_input).await;
        assert_eq!(urls.is_empty(), false);
    }

    #[test]
    fn test_get_version() {
        assert_eq!(get_version("1.0.0"), "1.0.0");
        assert_eq!(get_version("apache-maven-2.0.10-bin.tar.gz"), "2.0.10");
        assert_eq!(get_version("maven-1.0-beta-10.tar.gz"), "1.0-beta-10");
        assert_eq!(get_version("maven-2.0-alpha-2-bin.tar.gz"), "2.0-alpha-2");
    }
}
