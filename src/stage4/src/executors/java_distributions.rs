use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;
use serde::Serialize;

use crate::executor::{Download, GgVersion};
use crate::fetch::fetch_json;
use crate::github_utils::{
    create_github_client, detect_arch_from_name, detect_os_from_name, record_github_error,
};
use crate::target::{Arch, Os, Target, Variant};

type DistributionHandler = fn(&Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>>;

#[derive(Debug, Clone)]
pub struct DistributionConfig {
    pub name: &'static str,
    pub short_name: &'static str,
    pub default_tags: Vec<&'static str>,
    pub handler: DistributionHandler,
}

pub struct JavaDistributions;

const DEFAULT_DISTRIBUTION: &str = "temurin";

/// Adoptium only publishes the current LTS and recent feature releases, so `java@14` and
/// friends 404 there. Azul still carries them, so it backs up the default.
pub const FALLBACK_DISTRIBUTION: &str = "azul";

impl JavaDistributions {
    pub fn get_all() -> Vec<DistributionConfig> {
        vec![
            DistributionConfig {
                name: "temurin",
                short_name: "tem",
                default_tags: vec!["jdk", "ga"],
                handler: get_temurin_downloads,
            },
            DistributionConfig {
                name: "azul",
                short_name: "azul",
                default_tags: vec!["jdk", "ga"],
                handler: get_azul_downloads,
            },
            DistributionConfig {
                name: "graalvm",
                short_name: "graal",
                default_tags: vec!["jdk", "ga"],
                handler: get_graalvm_downloads,
            },
        ]
    }

    pub fn get_by_name(name: &str) -> Option<DistributionConfig> {
        Self::get_all()
            .into_iter()
            .find(|dist| dist.name == name || dist.short_name == name)
    }

    pub fn get_default() -> DistributionConfig {
        Self::get_by_name(DEFAULT_DISTRIBUTION).expect("default distribution must be registered")
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
    let target = *target;
    Box::pin(async move {
        // Azul backs the default now, not just an explicit -azul, so a bad day at
        // admin-ajax.php (it likes answering with HTML) must not panic the whole run
        let bundles: Vec<AzulBundle> = match fetch_json("https://www.azul.com/wp-admin/admin-ajax.php?action=bundles&endpoint=community&use_stage=false&include_fields=java_version,release_status,abi,arch,bundle_type,cpu_gen,ext,features,hw_bitness,javafx,latest,os,support_term").await {
            Some(bundles) => bundles,
            None => return vec![],
        };

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

/// Used when api.adoptium.net/v3/info/available_releases can't be reached or parsed.
const TEMURIN_FALLBACK_VERSIONS: [u32; 5] = [8, 11, 17, 21, 25];

async fn get_temurin_available_releases() -> Option<TemurinAvailableReleases> {
    let text = reqwest::get("https://api.adoptium.net/v3/info/available_releases")
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    serde_json::from_str(&text).ok()
}

async fn get_temurin_version_downloads(target: Target, version: u32, lts: bool) -> Vec<Download> {
    let url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?page_size=20&page=0&jvm_impl=hotspot&vendor=eclipse",
        version
    );

    let mut downloads = Vec::new();

    if let Ok(response) = reqwest::get(&url).await {
        if let Ok(text) = response.text().await {
            if let Ok(releases) = serde_json::from_str::<Vec<TemurinRelease>>(&text) {
                for release in releases {
                    for binary in release.binaries {
                        // Adoptium also ships sbom/testimage/debugimage/sources/staticlibs,
                        // which nobody can run. jre stays - image_type becomes a tag, so
                        // java@-jdk+jre can ask for it
                        if binary.image_type != "jdk" && binary.image_type != "jre" {
                            continue;
                        }

                        let os_match = match (&target.os, binary.os.as_str()) {
                            (Os::Windows, "windows") => true,
                            (Os::Linux, "linux") => target.variant != Some(Variant::Musl),
                            (Os::Linux, "alpine-linux") => target.variant == Some(Variant::Musl),
                            (Os::Mac, "mac") => true,
                            (Os::Any, _) => true,
                            _ => false,
                        };

                        let arch_match = matches!(
                            (&target.arch, binary.architecture.as_str()),
                            (Arch::X86_64, "x64")
                                | (Arch::X86_64, "x86_64")
                                | (Arch::Arm64, "aarch64")
                                | (Arch::Arm64, "arm64")
                                | (Arch::Armv7, "arm")
                                | (Arch::Any, _)
                        );

                        if os_match && arch_match {
                            let mut tags = HashSet::new();
                            tags.insert(binary.image_type.clone());
                            tags.insert(release.release_type.clone());
                            tags.insert(binary.heap_size.clone());
                            tags.insert(binary.jvm_impl.clone());
                            tags.insert(format!("java{}", version));

                            if lts {
                                tags.insert("lts".to_string());
                            }

                            let os = match binary.os.as_str() {
                                "windows" => Some(Os::Windows),
                                "linux" => Some(Os::Linux),
                                "alpine-linux" => Some(Os::Linux),
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

                            let variant = if binary.os == "alpine-linux" {
                                Some(Variant::Musl)
                            } else {
                                target.variant
                            };

                            downloads.push(Download {
                                download_url: binary.package.link,
                                version: GgVersion::new(&release.version_data.semver),
                                os,
                                arch,
                                variant,
                                tags,
                            });
                        }
                    }
                }
            }
        }
    }

    downloads
}

fn get_temurin_downloads(target: &Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>> {
    let target = *target;
    Box::pin(async move {
        let (versions, lts_versions) = match get_temurin_available_releases().await {
            Some(available) => {
                let mut versions = available.available_releases;
                if !versions.contains(&available.most_recent_feature_release) {
                    versions.push(available.most_recent_feature_release);
                }
                (versions, available.available_lts_releases)
            }
            None => (
                TEMURIN_FALLBACK_VERSIONS.to_vec(),
                TEMURIN_FALLBACK_VERSIONS.to_vec(),
            ),
        };

        // One request per feature release, so fan them out instead of paying for them in sequence.
        futures_util::future::join_all(versions.into_iter().map(|version| {
            get_temurin_version_downloads(target, version, lts_versions.contains(&version))
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    })
}

/// `graalvm-community-jdk-<version>_<os>-<arch>_bin.tar.gz`, `.zip` on Windows.
///
/// The version has to come from the asset and not the release tag: the newest ones
/// are tagged `graal-25.2.4`, which says nothing about the JDK inside (25.0.4).
fn graalvm_download(asset_name: &str, download_url: &str) -> Option<Download> {
    if !asset_name.ends_with("_bin.tar.gz") && !asset_name.ends_with("_bin.zip") {
        return None;
    }

    let (version_part, _) = asset_name
        .strip_prefix("graalvm-community-jdk-")?
        .split_once('_')?;
    // find_version already picks 25.0.4 out of 25i2-25.0.4. Don't split on the dash
    // first, that would throw away a future 24.0.1-b01 instead of reading it
    let version = GgVersion::new(version_part)?;

    let os = detect_os_from_name(asset_name)?;
    let arch = detect_arch_from_name(asset_name)?;

    // jdk and ga are required defaults - without both, executor.rs drops everything
    let tags = HashSet::from([
        "jdk".to_string(),
        "ga".to_string(),
        "graalvm".to_string(),
        format!("java{}", version.to_version().major),
    ]);

    Some(Download {
        download_url: download_url.to_string(),
        version: Some(version),
        os: Some(os),
        arch: Some(arch),
        // CE has no musl build, so alpine gets the glibc one and finds out at runtime
        variant: Some(Variant::Any),
        tags,
    })
}

/// Prereleases and drafts have to go before the assets pick up the `ga` tag below,
/// or an EA build ends up claiming to be a GA one.
fn graalvm_release_downloads<'a>(
    prerelease: bool,
    draft: bool,
    assets: impl IntoIterator<Item = (&'a str, &'a str)>,
    target: &Target,
) -> Vec<Download> {
    if prerelease || draft {
        return vec![];
    }

    assets
        .into_iter()
        .filter_map(|(name, url)| graalvm_download(name, url))
        .filter(|download| download.os == Some(target.os) && download.arch == Some(target.arch))
        .collect()
}

/// No install step needed - `native-image` is bundled in every asset we accept. The
/// `gu` era ended with the `graalvm-community-jdk-*` naming at 17.0.7, not at 21.
fn get_graalvm_downloads(target: &Target) -> Pin<Box<dyn Future<Output = Vec<Download>> + Send>> {
    let target = *target;
    Box::pin(async move {
        let mut downloads: Vec<Download> = vec![];

        let octocrab = match create_github_client() {
            Ok(octocrab) => octocrab,
            Err(err) => {
                // Silence here reads as "no build for your platform" further down
                record_github_error("graalvm/graalvm-ce-builds", &err);
                return downloads;
            }
        };

        let mut page: u32 = 1;
        loop {
            let releases_result = octocrab
                .repos("graalvm", "graalvm-ce-builds")
                .releases()
                .list()
                .page(page)
                .per_page(100)
                .send()
                .await;

            match releases_result {
                Ok(releases) => {
                    for release in releases.items {
                        downloads.extend(graalvm_release_downloads(
                            release.prerelease,
                            release.draft,
                            release
                                .assets
                                .iter()
                                .map(|a| (a.name.as_str(), a.browser_download_url.as_str())),
                            &target,
                        ));
                    }

                    if releases.next.is_none() {
                        break;
                    }
                    page += 1;
                }
                Err(err) => {
                    record_github_error("graalvm/graalvm-ce-builds", &err);
                    break;
                }
            }
        }

        downloads
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn download_for(asset_name: &str) -> Option<Download> {
        graalvm_download(asset_name, &format!("http://x/{asset_name}"))
    }

    fn version_of(asset_name: &str) -> String {
        download_for(asset_name)
            .unwrap()
            .version
            .unwrap()
            .to_version()
            .to_string()
    }

    #[test]
    fn test_graalvm_is_registered_under_both_names() {
        assert_eq!(
            JavaDistributions::get_by_name("graalvm").map(|d| d.name),
            Some("graalvm")
        );
        // java@21-graal
        assert_eq!(
            JavaDistributions::get_by_name("graal").map(|d| d.name),
            Some("graalvm")
        );
    }

    #[test]
    fn test_graalvm_version_comes_from_the_asset_not_the_tag() {
        assert_eq!(
            version_of("graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz"),
            "25.0.2"
        );
        // Tagged graal-25.2.4, so the tag alone would say 25.2.4 and java@25.0.4-graal
        // would miss the newest build there is
        assert_eq!(
            version_of("graalvm-community-jdk-25i2-25.0.4_linux-x64_bin.tar.gz"),
            "25.0.4"
        );
        assert_eq!(
            version_of("graalvm-community-jdk-25i1-25.0.3_linux-x64_bin.tar.gz"),
            "25.0.3"
        );
    }

    #[test]
    fn test_graalvm_interim_build_outranks_the_older_plain_release() {
        let interim = download_for("graalvm-community-jdk-25i2-25.0.4_linux-x64_bin.tar.gz")
            .unwrap()
            .version
            .unwrap()
            .to_version();
        let plain = download_for("graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz")
            .unwrap()
            .version
            .unwrap()
            .to_version();
        assert!(interim > plain, "{} should sort above {}", interim, plain);
    }

    #[test]
    fn test_graalvm_asset_names_resolve_to_a_platform() {
        let cases = [
            (
                "graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz",
                Os::Linux,
                Arch::X86_64,
            ),
            (
                "graalvm-community-jdk-25.0.2_linux-aarch64_bin.tar.gz",
                Os::Linux,
                Arch::Arm64,
            ),
            (
                "graalvm-community-jdk-25.0.2_macos-aarch64_bin.tar.gz",
                Os::Mac,
                Arch::Arm64,
            ),
            // dropped at 25.0.2, but older releases still carry it
            (
                "graalvm-community-jdk-24.0.2_macos-x64_bin.tar.gz",
                Os::Mac,
                Arch::X86_64,
            ),
            (
                "graalvm-community-jdk-25.0.2_windows-x64_bin.zip",
                Os::Windows,
                Arch::X86_64,
            ),
        ];

        for (name, os, arch) in cases {
            let download = download_for(name).unwrap_or_else(|| panic!("no download for {}", name));
            assert_eq!(download.os, Some(os), "os for {}", name);
            assert_eq!(download.arch, Some(arch), "arch for {}", name);
            assert_eq!(download.variant, Some(Variant::Any), "variant for {}", name);
        }
    }

    #[test]
    fn test_graalvm_carries_the_required_default_tags() {
        let tags = download_for("graalvm-community-jdk-25i2-25.0.4_linux-x64_bin.tar.gz")
            .unwrap()
            .tags;
        assert!(tags.contains("jdk"));
        assert!(tags.contains("ga"));
        assert!(tags.contains("graalvm"));
        assert!(tags.contains("java25"));
    }

    fn target_for(os: Os, arch: Arch) -> Target {
        Target {
            os,
            arch,
            variant: None,
        }
    }

    /// One real release page: four platforms, one JDK.
    const RELEASE_ASSETS: [(&str, &str); 4] = [
        (
            "graalvm-community-jdk-25i2-25.0.4_linux-x64_bin.tar.gz",
            "http://x/linux-x64",
        ),
        (
            "graalvm-community-jdk-25i2-25.0.4_linux-aarch64_bin.tar.gz",
            "http://x/linux-aarch64",
        ),
        (
            "graalvm-community-jdk-25i2-25.0.4_macos-aarch64_bin.tar.gz",
            "http://x/macos-aarch64",
        ),
        (
            "graalvm-community-jdk-25i2-25.0.4_windows-x64_bin.zip",
            "http://x/windows-x64",
        ),
    ];

    #[test]
    fn test_graalvm_release_keeps_only_the_running_platform() {
        let downloads = graalvm_release_downloads(
            false,
            false,
            RELEASE_ASSETS.iter().copied(),
            &target_for(Os::Linux, Arch::X86_64),
        );

        assert_eq!(downloads.len(), 1, "one asset per platform per release");
        assert_eq!(downloads[0].download_url, "http://x/linux-x64");
        // the JDK version, not the graal-25.2.4 tag the release carries
        assert_eq!(
            downloads[0]
                .version
                .as_ref()
                .unwrap()
                .to_version()
                .to_string(),
            "25.0.4"
        );
    }

    #[test]
    fn test_graalvm_release_has_nothing_for_a_platform_ce_skips() {
        // CE publishes no windows-aarch64, and 25.0.2 onward no macos-x64 either
        let downloads = graalvm_release_downloads(
            false,
            false,
            RELEASE_ASSETS.iter().copied(),
            &target_for(Os::Windows, Arch::Arm64),
        );
        assert!(downloads.is_empty());
    }

    #[test]
    fn test_graalvm_release_skips_prereleases_and_drafts() {
        let linux = target_for(Os::Linux, Arch::X86_64);
        assert!(
            graalvm_release_downloads(true, false, RELEASE_ASSETS.iter().copied(), &linux)
                .is_empty(),
            "prerelease"
        );
        assert!(
            graalvm_release_downloads(false, true, RELEASE_ASSETS.iter().copied(), &linux)
                .is_empty(),
            "draft"
        );
        assert_eq!(
            graalvm_release_downloads(false, false, RELEASE_ASSETS.iter().copied(), &linux).len(),
            1,
            "the same page is kept when the release is stable"
        );
    }

    #[test]
    fn test_graalvm_release_drops_sidecars_without_dropping_the_release() {
        let assets = [
            (
                "graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz.sha256",
                "http://x/sha",
            ),
            (
                "graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz",
                "http://x/real",
            ),
        ];
        let downloads = graalvm_release_downloads(
            false,
            false,
            assets.iter().copied(),
            &target_for(Os::Linux, Arch::X86_64),
        );

        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].download_url, "http://x/real");
    }

    #[test]
    fn test_graalvm_skips_what_it_cannot_run() {
        // checksum sidecar sits right next to the real asset
        assert!(download_for("graalvm-community-jdk-25.0.2_linux-x64_bin.tar.gz.sha256").is_none());
        // the pre-2023 naming
        assert!(download_for("graalvm-ce-java17-linux-amd64-22.3.0.tar.gz").is_none());
        // not ours at all
        assert!(download_for("OpenJDK21U-jdk_x64_linux_hotspot_21.0.2_13.tar.gz").is_none());
        // Oracle GraalVM, same suffix and shape as CE but a different license
        assert!(download_for("graalvm-jdk-25.0.2_linux-x64_bin.tar.gz").is_none());
    }
}
