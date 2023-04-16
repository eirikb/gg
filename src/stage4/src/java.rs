use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::pin::Pin;

use java_properties::read;
use semver::{Version, VersionReq};
use serde::Deserialize;
use serde::Serialize;

use crate::Executor;
use crate::executor::{AppInput, AppPath, Download, ExecutorCmd};
use crate::target::{Arch, Os, Target, Variant};

type Root = Vec<Root2>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Root2 {
    pub abi: String,
    pub arch: String,
    #[serde(rename = "bundle_type")]
    pub bundle_type: String,
    #[serde(rename = "cpu_gen")]
    pub cpu_gen: Vec<String>,
    pub ext: String,
    pub features: Vec<String>,
    #[serde(rename = "hw_bitness")]
    pub hw_bitness: String,
    pub id: i64,
    #[serde(rename = "java_version")]
    pub java_version: Vec<i64>,
    pub javafx: bool,
    #[serde(rename = "jdk_version")]
    pub jdk_version: Vec<i64>,
    pub latest: bool,
    pub name: String,
    #[serde(rename = "openjdk_build_number")]
    pub openjdk_build_number: Option<i64>,
    pub os: String,
    #[serde(rename = "release_status")]
    pub release_status: String,
    #[serde(rename = "support_term")]
    pub support_term: String,
    pub url: String,
}

pub struct Java {
    pub executor_cmd: ExecutorCmd,
}

fn get_jdk_version() -> Option<String> {
    if let Ok(file) = File::open("gradle/wrapper/gradle-wrapper.properties") {
        if let Ok(map) = read(BufReader::new(file)) {
            return map.get("jdkVersion").map(|s| s.clone());
        }
    }
    if let Ok(file) = File::open("gradle.properties") {
        if let Ok(map) = read(BufReader::new(file)) {
            return map.get("jdkVersion").map(|s| s.clone());
        }
    }
    None
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

    fn get_download_urls<'a>(&self, input: &'a AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
        Box::pin(async move { get_java_download_urls(&input.target).await })
    }

    fn get_bin(&self, input: &AppInput) -> Vec<&str> {
        vec!(match &input.target.os {
            Os::Windows => "bin/java.exe",
            _ => "bin/java"
        })
    }

    fn get_name(&self) -> &str {
        "java"
    }

    fn get_default_include_tags(&self) -> HashSet<String> {
        vec!["jdk", "ga"].into_iter().map(|s| s.to_string()).collect()
    }

    fn get_env(&self, app_path: AppPath) -> HashMap<String, String> {
        [(String::from("JAVA_HOME"), app_path.app.to_str().unwrap().to_string())].iter().cloned().collect()
    }
}

async fn get_java_download_urls(_target: &Target) -> Vec<Download> {
    let json = reqwest::get("https://www.azul.com/wp-admin/admin-ajax.php?action=bundles&endpoint=community&use_stage=false&include_fields=java_version,release_status,abi,arch,bundle_type,cpu_gen,ext,features,hw_bitness,javafx,latest,os,support_term").await.unwrap().text().await.unwrap();
    let root: Root = serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");
    root.iter().map(|node| {
        let n = node.clone();
        let mut tags = HashSet::new();
        tags.insert(n.bundle_type);
        tags.insert(n.support_term);
        tags.insert(n.release_status);

        for feature in n.features {
            tags.insert(feature);
        }
        let os = Some(match node.os.as_str() {
            "windows" => Os::Windows,
            x if x.contains("linux") => Os::Linux,
            _ => Os::Mac,
        });
        let arch = match (node.arch.as_str(), node.hw_bitness.as_str()) {
            ("x86", "64") => Some(Arch::X86_64),
            ("arm", "64") => Some(Arch::Armv7),
            _ => None
        };
        let variant = if node.os.as_str().contains("musl") {
            Some(Variant::Musl)
        } else {
            None
        };
        // TODO: ext?!
        // let ext = match target.os {
        //     Os::Windows => "zip",
        //     _ => "tar.gz",
        // };
        Download {
            download_url: n.url,
            version:
            Version::parse(&n.java_version.into_iter().map(|i| i.to_string()).collect::<Vec<String>>().join(".")).ok(),
            os,
            arch,
            variant,
            tags,
        }
    }).collect()
}
