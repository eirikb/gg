use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::download_unpack_and_all_that_stuff;
use crate::executor::{prep_bin, try_run};
use crate::target::{Arch, Os, Target, Variant};

pub type Root = Vec<Root2>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root2 {
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

async fn prep(target: Target) -> () {
    let java_url = get_java_download_url(&target).await;
    println!("Java download url: {}", java_url);
    download_unpack_and_all_that_stuff(&java_url, ".cache/java").await;
}

pub async fn prep_java(target: Target) -> Result<PathBuf, String> {
    let bin = match &target.os {
        Os::Windows => "bin/java.exe",
        _ => "bin/java"
    };
    prep_bin(bin, "java", || Box::pin(prep(target))).await
}

pub async fn try_run_java(target: Target) -> Result<(), String> {
    let bin_path = prep_java(target).await?.clone();
    println!("path is {:?}", bin_path);
    if bin_path.exists() {
        return if try_run(bin_path.to_str().unwrap_or("")).unwrap() {
            Ok(())
        } else {
            Err("Unable to execute".to_string())
        };
    }
    Ok(())
}

pub async fn get_java_download_url(target: &Target) -> String {
    let json = reqwest::get("https://www.azul.com/wp-admin/admin-ajax.php?action=bundles&endpoint=community&use_stage=false&include_fields=java_version,release_status,abi,arch,bundle_type,cpu_gen,ext,features,hw_bitness,javafx,latest,os,support_term").await.unwrap().text().await.unwrap();
    let root: Root = serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");
    let node = root.iter().find(|node| {
        let node_os = match node.os.as_str() {
            "windows" => Os::Windows,
            x if x.contains("linux") => Os::Linux,
            _ => Os::Mac,
        };
        let ext = match target.os {
            Os::Windows => "zip",
            _ => "tar.gz",
        };
        let node_arch = match (node.arch.as_str(), node.hw_bitness.as_str()) {
            ("x86", "64") => Some(Arch::X86_64),
            ("arm", "64") => Some(Arch::Armv7),
            _ => None
        };
        let variant_check = target.variant != Variant::Musl || node.os.as_str().contains("musl");
        if node_arch.is_some() {
            variant_check && node_os == target.os && node_arch.unwrap() == target.arch && node.ext == ext
        } else {
            false
        }
    });
    return String::from(node.unwrap().clone().url);
}
