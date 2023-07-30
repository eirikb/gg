use std::fs;

use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleWrapperProperties {
    pub distribution_url: Option<String>,
    pub jdk_version: Option<String>,
    pub distribution_sha256sum: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleProperties {
    pub distribution_url: Option<String>,
    pub jdk_version: Option<String>,
}

pub struct GradleAndWrapperProperties {
    pub gradle_properties: Option<GradleProperties>,
    pub gradle_wrapper_properties: Option<GradleWrapperProperties>,
}

fn get_version_from_gradle_url(gradle_url: &str) -> Option<String> {
    if let Ok(r) = Regex::new(r"gradle-(.*)-") {
        let captures: Vec<_> = r.captures_iter(gradle_url).collect();
        if captures.len() > 0 {
            if let Some(cap) = captures[0].get(1) {
                return Some(cap.as_str().to_string());
            }
        }
    }
    None
}

impl GradleAndWrapperProperties {
    pub fn new() -> GradleAndWrapperProperties {
        GradleAndWrapperProperties {
            gradle_properties: fs::read_to_string("gradle.properties").ok()
                .and_then(|text| serde_java_properties::from_str(text.as_str()).ok()),
            gradle_wrapper_properties: fs::read_to_string("gradle/wrapper/gradle-wrapper.properties").ok()
                .and_then(|text| serde_java_properties::from_str(text.as_str()).ok()),
        }
    }

    fn map<F, G, U>(&self, gradle_wrapper_extractor: F, gradle_extractor: G) -> Option<U>
        where
            F: Fn(&GradleWrapperProperties) -> Option<U>,
            G: Fn(&GradleProperties) -> Option<U>,
    {
        self.gradle_wrapper_properties.as_ref().and_then(&gradle_wrapper_extractor)
            .or(self.gradle_properties.as_ref().and_then(&gradle_extractor))
    }

    pub fn get_version_from_distribution_url(&self) -> Option<String> {
        self.get_distribution_url().map(|url| get_version_from_gradle_url(url.as_str())).flatten()
    }

    pub fn get_distribution_url(&self) -> Option<String> {
        self.map(|p| p.distribution_url.clone(), |p| p.distribution_url.clone())
    }

    pub fn get_jdk_version(&self) -> Option<String> {
        self.map(|p| p.jdk_version.clone(), |p| p.jdk_version.clone())
    }

    pub fn get_distribution_sha256sum(&self) -> Option<String> {
        self.gradle_wrapper_properties.as_ref().and_then(|p| p.distribution_sha256sum.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::executors::gradle_properties::{get_version_from_gradle_url, GradleProperties, GradleWrapperProperties};

    #[test]
    fn test_get_version_from_distribution_url() {
        let input = "https://services.gradle.org/distributions/gradle-6.8.3-bin.zip";
        let version = get_version_from_gradle_url(input);
        assert_eq!(version.unwrap(), "6.8.3");
    }

    #[test]
    fn test_gradle_wrapper_properties() {
        let text = "\
distributionUrl=https:\\//example.com
jdkVersion=v1
        ";

        let props: GradleWrapperProperties = serde_java_properties::from_str(text).unwrap();

        assert_eq!(props.distribution_url, Some("https://example.com".to_string()));
        assert_eq!(props.jdk_version, Some("v1".to_string()));
    }

    #[test]
    fn test_gradle_properties() {
        let text = "\
distributionUrl=https\\://example.com
jdkVersion=v1
        ";

        let props: GradleProperties = serde_java_properties::from_str(text).unwrap();

        assert_eq!(props.distribution_url, Some("https://example.com".to_string()));
        assert_eq!(props.jdk_version, Some("v1".to_string()));
    }
}
