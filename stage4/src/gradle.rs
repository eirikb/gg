use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::pin::Pin;
use std::process::Command;

use java_properties::read;
use regex::Regex;
use scraper::{Html, Selector};
use semver::VersionReq;

use crate::{Executor, Java};
use crate::executor::{AppInput, Download, prep};

use super::target;

pub struct Gradle {
    pub version_req_map: HashMap<String, Option<VersionReq>>,
}

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

impl Executor for Gradle {
    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(v) = self.version_req_map.get("gradle") {
            if let Some(v) = v {
                return Some(v.clone());
            }
        }
        if let Ok(file) = File::open("gradle/wrapper/gradle-wrapper.properties") {
            if let Ok(map) = read(BufReader::new(file)) {
                if let Some(distribution_url) = map.get("distributionUrl") {
                    if let Some(version) = distribution_url.get_version_from_gradle_url() {
                        return VersionReq::parse(version.as_str()).ok();
                    }
                }
            }
        }
        None
    }

    fn get_download_urls(&self, _input: &AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>>>> {
        Box::pin(async move {
            let body = reqwest::get("https://gradle.org/releases").await
                .expect("Unable to connect to services.gradle.org").text().await
                .expect("Unable to download gradle list of versions");

            let document = Html::parse_document(body.as_str());
            document.select(&Selector::parse("a[name]").unwrap()).map(|link| {
                let version = link.value().attr("name").unwrap_or("").to_string();
                Download {
                    download_url: format!("https://services.gradle.org/distributions/gradle-{version}-bin.zip"),
                    lts: false,
                    version,
                }
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

    fn before_exec<'a>(&'a self, input: &'a AppInput, command: &'a mut Command) -> Pin<Box<dyn Future<Output=Option<String>> + 'a>> {
        Box::pin(async move {
            let app_path = prep(&Java { version_req_map: self.version_req_map.clone() }, input).await.expect("Unable to install Java");
            println!("java path is {:?}", app_path);
            command.env("JAVA_HOME", app_path.app.clone());
            let path_string = &env::var("PATH").unwrap_or("".to_string());
            let parent_bin_path = app_path.parent_bin_path();
            let path = format!("{parent_bin_path}:{path_string}");
            println!("PATH: {path}");
            Some(path)
        })
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
