use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;
use serde::Serialize;

use crate::executor::{Download, GgVersion};
use crate::target::{Arch, Os, Target, Variant};

#[derive(Debug, Clone)]
pub struct DistributionConfig {
    pub name: &'static str,
    pub short_name: &'static str,
    pub default_tags: Vec<&'static str>,
    pub handler: fn(&Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>>,
}

pub struct JavaDistributions;

impl JavaDistributions {
    pub fn get_all() -> Vec<DistributionConfig> {
        vec![
            DistributionConfig {
                name: "azul",
                short_name: "azul",
                default_tags: vec!["jdk", "ga"],
                handler: get_azul_downloads,
            },
            DistributionConfig {
                name: "temurin",
                short_name: "tem",
                default_tags: vec!["jdk", "ga"],
                handler: get_temurin_downloads,
            },
        ]
    }

    pub fn get_by_name(name: &str) -> Option<DistributionConfig> {
        Self::get_all()
            .into_iter()
            .find(|dist| dist.name == name || dist.short_name == name)
    }

    pub fn get_default() -> DistributionConfig {
        Self::get_all().into_iter().next().unwrap()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzulBundle {
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

fn get_azul_downloads(target: &Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>> {
    let target = target.clone();
    Box::pin(async move {
        let json = reqwest::get("https://www.azul.com/wp-admin/admin-ajax.php?action=bundles&endpoint=community&use_stage=false&include_fields=java_version,release_status,abi,arch,bundle_type,cpu_gen,ext,features,hw_bitness,javafx,latest,os,support_term").await.unwrap().text().await.unwrap();
        let bundles: Vec<AzulBundle> =
            serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");

        bundles
            .iter()
            .filter(|node| match target.os {
                Os::Windows => node.ext == "zip",
                _ => node.ext == "tar.gz",
            })
            .map(|node| {
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
                    ("arm", "32") => Some(Arch::Armv7),
                    ("arm", "64") => Some(Arch::Arm64),
                    _ => None,
                };
                let variant = if node.os.as_str().contains("musl") {
                    Some(Variant::Musl)
                } else {
                    None
                };
                Download {
                    download_url: n.url,
                    version: GgVersion::new(
                        &n.java_version
                            .into_iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<String>>()
                            .join("."),
                    ),
                    os,
                    arch,
                    variant,
                    tags,
                }
            })
            .collect()
    })
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TemurinAvailableReleases {
    pub available_lts_releases: Vec<u32>,
    pub available_releases: Vec<u32>,
    pub most_recent_feature_release: u32,
    pub most_recent_feature_version: u32,
    pub most_recent_lts: u32,
    pub tip_version: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TemurinRelease {
    pub binaries: Vec<TemurinBinary>,
    pub version_data: TemurinVersionData,
    pub release_name: String,
    pub release_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TemurinBinary {
    pub architecture: String,
    pub download_count: u64,
    pub heap_size: String,
    pub image_type: String,
    pub installer: Option<serde_json::Value>,
    pub jvm_impl: String,
    pub os: String,
    pub package: TemurinPackage,
    pub project: String,
    pub scm_ref: String,
    pub updated_at: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TemurinPackage {
    pub checksum: Option<String>,
    pub checksum_link: Option<String>,
    pub download_count: u64,
    pub link: String,
    pub metadata_link: Option<String>,
    pub name: String,
    pub signature_link: Option<String>,
    pub size: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TemurinVersionData {
    pub build: u32,
    pub major: u32,
    pub minor: u32,
    pub openjdk_version: String,
    pub optional: Option<String>,
    pub security: u32,
    pub semver: String,
}

fn get_temurin_downloads(target: &Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>> {
    let target = target.clone();
    Box::pin(async move {
        let mut downloads = Vec::new();

        let available_releases_url = "https://api.adoptium.net/v3/info/available_releases";
        let versions_to_fetch = match reqwest::get(available_releases_url).await {
            Ok(response) => match response.text().await {
                Ok(text) => match serde_json::from_str::<TemurinAvailableReleases>(&text) {
                    Ok(available) => {
                        let mut versions = available.available_lts_releases;
                        if !versions.contains(&available.most_recent_feature_release) {
                            versions.push(available.most_recent_feature_release);
                        }
                        versions
                    }
                    Err(_) => vec![8, 11, 17, 21],
                },
                Err(_) => vec![8, 11, 17, 21],
            },
            Err(_) => vec![8, 11, 17, 21],
        };

        for version in versions_to_fetch {
            let url = format!(
                "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?page_size=20&page=0&jvm_impl=hotspot&vendor=eclipse",
                version
            );

            if let Ok(response) = reqwest::get(&url).await {
                if let Ok(text) = response.text().await {
                    if let Ok(releases) = serde_json::from_str::<Vec<TemurinRelease>>(&text) {
                        for release in releases {
                            for binary in release.binaries {
                                if binary.image_type != "jdk" {
                                    continue;
                                }

                                let os_match = match (&target.os, binary.os.as_str()) {
                                    (Os::Windows, "windows") => true,
                                    (Os::Linux, "linux") => true,
                                    (Os::Mac, "mac") => true,
                                    (Os::Any, _) => true,
                                    _ => false,
                                };

                                let arch_match = match (&target.arch, binary.architecture.as_str())
                                {
                                    (Arch::X86_64, "x64") => true,
                                    (Arch::X86_64, "x86_64") => true,
                                    (Arch::Arm64, "aarch64") => true,
                                    (Arch::Arm64, "arm64") => true,
                                    (Arch::Armv7, "arm") => true,
                                    (Arch::Any, _) => true,
                                    _ => false,
                                };

                                if os_match && arch_match {
                                    let mut tags = HashSet::new();
                                    tags.insert(binary.image_type.clone());
                                    tags.insert(release.release_type.clone());
                                    tags.insert(binary.heap_size.clone());
                                    tags.insert(binary.jvm_impl.clone());
                                    tags.insert(format!("java{}", version));

                                    if [8, 11, 17, 21].contains(&version) {
                                        tags.insert("lts".to_string());
                                    }

                                    let os = match binary.os.as_str() {
                                        "windows" => Some(Os::Windows),
                                        "linux" => Some(Os::Linux),
                                        "mac" => Some(Os::Mac),
                                        _ => None,
                                    };

                                    let arch = match binary.architecture.as_str() {
                                        "x64" | "x86_64" => Some(Arch::X86_64),
                                        "aarch64" | "arm64" => Some(Arch::Arm64),
                                        "arm" => Some(Arch::Armv7),
                                        "x86" | "x32" => None,
                                        _ => None,
                                    };

                                    downloads.push(Download {
                                        download_url: binary.package.link,
                                        version: GgVersion::new(&release.version_data.semver),
                                        os,
                                        arch,
                                        variant: target.variant.clone(),
                                        tags,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        downloads
    })
}
