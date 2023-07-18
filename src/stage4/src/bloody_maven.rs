use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;

use crate::executor::{Download, GgVersion};
use crate::target::{Arch, Os, Variant};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Item {
    name: String,
    source: String,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "groupId")]
    group_id: String,

    #[serde(rename = "artifactId")]
    artifact_id: String,

    #[serde(rename = "versioning")]
    versioning: Versioning,
}

#[derive(Serialize, Deserialize)]
pub struct Versioning {
    #[serde(rename = "latest")]
    latest: String,

    #[serde(rename = "release")]
    release: String,

    #[serde(rename = "versions")]
    versions: Versions,

    #[serde(rename = "lastUpdated")]
    last_updated: String,
}

#[derive(Serialize, Deserialize)]
pub struct Versions {
    #[serde(rename = "version")]
    version: Vec<String>,
}

pub fn get_download_urls_from_maven<'a>(group: &'a str, artifact: &'a str) -> Pin<Box<dyn Future<Output=Vec<Download>> + 'a>> {
    Box::pin(async move {
        let root_url = format!("https://repo1.maven.org/maven2/org/{group}/{artifact}");
        let metadata_url = format!("{root_url}/maven-metadata.xml");
        let body = reqwest::get(metadata_url.clone()).await
            .expect("Unable to connect to archive.apache.org").text().await
            .expect("Unable to download maven metadata xml");
        let root: Metadata = from_str(body.as_str()).expect("XML was not well-formatted");

        root.versioning.versions.version.into_iter().map(|ver| {
            let mut tags = HashSet::new();
            if ver.contains("beta") {
                tags.insert("beta".to_string());
            }
            Download {
                download_url: format!("{root_url}/{ver}/{artifact}-{ver}.jar"),
                version: GgVersion::new(ver.as_str()),
                os: Some(Os::Any),
                arch: Some(Arch::Any),
                variant: Some(Variant::Any),
                tags,
            }
        }).collect()
    })
}
