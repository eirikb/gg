use std::future::Future;
use std::pin::Pin;

use scraper::{Html, Selector};

use crate::Executor;
use crate::executor::{AppInput, Download, ExecutorCmd};
use crate::target::Variant;

use super::target;

pub struct Maven {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Maven {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        return &self.executor_cmd;
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move {
            let body = reqwest::get("https://maven.apache.org/docs/history.html").await
                .expect("Unable to connect to maven.apache.org").text().await
                .expect("Unable to download maven list of versions");

            let document = Html::parse_document(body.as_str());
            document.select(&Selector::parse("tr td:nth-child(2)").unwrap()).map(|td| {
                let version = td.text().next().unwrap_or("").trim();
                let major = version.chars().next().unwrap_or('0');
                Download::new(
                    format!("https://dlcdn.apache.org/maven/maven-{major}/{version}/binaries/apache-maven-{version}-bin.tar.gz"),
                    version,
                    Some(Variant::Any),
                )
            }).collect()
        })
    }

    fn get_bin(&self, input: &AppInput) -> &str {
        match &input.target.os {
            target::Os::Windows => "bin/mvn.cmd",
            _ => "bin/mvn"
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
}
