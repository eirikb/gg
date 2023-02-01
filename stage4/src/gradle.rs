use std::collections::HashSet;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::pin::Pin;

use java_properties::read;
use regex::Regex;
use scraper::{Html, Selector};
use semver::{Version, VersionReq};

use crate::{Executor};
use crate::executor::{AppInput, Download};
use crate::version::GGVersion;

use super::target;

pub struct Gradle {}

trait HelloWorld {
    fn get_version_from_gradle_url(&self) -> Option<String>;
}

impl HelloWorld for String {
    fn get_version_from_gradle_url(&self) -> Option<String> {
        if let Ok(r) = Regex::new(r"gradle-(.*)-") {
            let captures: Vec<_> = r.captures_iter(self).collect();
            if captures.len() > 0 {
                if let Some(cap) = captures[0].get(1) {
                    return Some(cap.as_str().to_string());
                }
            }
        }
        None
    }
}

fn get_distribution_url() -> Option<String> {
    if let Ok(file) = File::open("gradle/wrapper/gradle-wrapper.properties") {
        if let Ok(map) = read(BufReader::new(file)) {
            return map.get("distributionUrl").map(|s| s.clone());
        }
    }
    if let Ok(file) = File::open("gradle.properties") {
        if let Ok(map) = read(BufReader::new(file)) {
            return map.get("distributionUrl").map(|s| s.clone());
        }
    }
    None
}

impl Executor for Gradle {
    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(distribution_url) = get_distribution_url() {
            if let Some(version) = distribution_url.get_version_from_gradle_url() {
                return VersionReq::parse(version.as_str()).ok();
            }
        }
        None
    }

    fn get_download_urls<'a>(&'a self, _input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move {
            if let Some(distribution_url) = get_distribution_url() {
                if let Some(version) = distribution_url.get_version_from_gradle_url() {
                    return vec![
                        Download::new(distribution_url, version.as_str())
                    ];
                }
            }


            let body = reqwest::get("https://gradle.org/releases").await
                .expect("Unable to connect to services.gradle.org").text().await
                .expect("Unable to download gradle list of versions");

            let document = Html::parse_document(body.as_str());
            document.select(&Selector::parse("a[name]").unwrap()).map(|link| {
                let version = link.value().attr("name").unwrap_or("").to_string();
                Download::new(
                    format!("https://services.gradle.org/distributions/gradle-{version}-bin.zip"),
                    version.as_str(),
                )
            }).collect()
        })
    }

    fn get_bin(&self, input: &AppInput) -> &str {
        match &input.target.os {
            target::Os::Windows => "bin/gradle.exe",
            _ => "bin/gradle"
        }
    }

    fn get_name(&self) -> &str {
        "gradle"
    }

    fn get_deps(&self) -> Vec<&str> {
        vec!("java")
    }
}


#[cfg(test)]
mod tests {
    use crate::gradle::HelloWorld;

    #[test]
    fn it_works() {
        let input = "https://services.gradle.org/distributions/gradle-6.8.3-bin.zip";
        let version = input.to_string().get_version_from_gradle_url();
        assert_eq!(version.unwrap(), "6.8.3");
    }
}
