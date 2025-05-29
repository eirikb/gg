use scraper::{Html, Selector};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::Arch::{Arm64, X86_64};
use crate::target::Os::{Linux, Mac, Windows};
use crate::target::Variant::Any;

pub struct Go {
    pub executor_cmd: ExecutorCmd,
}

fn link_href_to_download(href: &str) -> Option<Download> {
    let href_part = href.replace("/dl/go", "");
    let supported_oses = vec![("linux", Linux), ("darwin", Mac), ("windows", Windows)];
    let supported_archs = vec![("amd64", X86_64), ("arm64", Arm64)];
    let supported_extensions = vec!["tar.gz", "zip"];

    if !supported_extensions
        .iter()
        .any(|ext| href_part.ends_with(ext))
    {
        return None;
    }

    if let Some((_, arch)) = supported_archs
        .into_iter()
        .find(|(arch_name, _)| href.contains(arch_name))
    {
        for (os_name, os) in supported_oses {
            if let Some(pos) = href_part.to_lowercase().find(os_name) {
                let version = href_part[0..pos - 1].to_string();

                let mut tags = HashSet::new();

                let version = if let Some(pos) = version.find("beta") {
                    tags.insert("beta".to_string());
                    version[0..pos].to_string()
                } else {
                    version
                };

                return Some(Download {
                    version: GgVersion::new(version.as_str()),
                    tags,
                    download_url: format!("https://go.dev{}", href),
                    arch: Some(arch),
                    os: Some(os),
                    variant: Some(Any),
                });
            }
        }
    }
    None
}

impl Executor for Go {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            // let mut downloads: Vec<Download> = vec!();
            let body = reqwest::get("https://go.dev/dl/")
                .await
                .expect("Unable to connect to go.dev")
                .text()
                .await
                .expect("Unable to download gradle list of versions");

            let document = Html::parse_document(body.as_str());
            let downloads: Vec<Download> = document
                .select(&Selector::parse("a.download").unwrap())
                .map(|link| {
                    return if let Some(href) = link.value().attr("href") {
                        link_href_to_download(href)
                    } else {
                        None
                    };
                })
                .filter_map(|document| document)
                .collect();
            downloads
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Windows => "go.exe",
                _ => "go",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "go"
    }

    fn get_default_exclude_tags(&self) -> HashSet<String> {
        vec!["beta".to_string()].into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::executor::{Download, GgVersion};
    use crate::executors::go::link_href_to_download;
    use crate::target::Os::{Linux, Mac, Windows};
    use crate::target::{Arch, Variant};
    use std::collections::HashSet;

    #[test]
    fn test_link_href_to_download() {
        let download = link_href_to_download("/dl/go1.16.5.windows-amd64.zip");
        assert_eq!(
            download,
            Some(Download {
                download_url: "https://go.dev/dl/go1.16.5.windows-amd64.zip".to_string(),
                version: GgVersion::new("1.16.5"),
                tags: HashSet::new(),
                arch: Some(Arch::X86_64),
                variant: Some(Variant::Any),
                os: Some(Windows),
            })
        );
    }

    #[test]
    fn test_link_href_to_download2() {
        let download = link_href_to_download("/dl/go1.20.6.linux-amd64.tar.gz");
        assert_eq!(
            download,
            Some(Download {
                download_url: "https://go.dev/dl/go1.20.6.linux-amd64.tar.gz".to_string(),
                version: GgVersion::new("1.20.6"),
                tags: HashSet::new(),
                arch: Some(Arch::X86_64),
                variant: Some(Variant::Any),
                os: Some(Linux),
            })
        );
    }

    #[test]
    fn test_link_href_to_download3() {
        let download = link_href_to_download("/dl/go1.20.6.linux-arm64.tar.gz");
        assert_eq!(
            download,
            Some(Download {
                download_url: "https://go.dev/dl/go1.20.6.linux-arm64.tar.gz".to_string(),
                version: GgVersion::new("1.20.6"),
                tags: HashSet::new(),
                arch: Some(Arch::Arm64),
                variant: Some(Variant::Any),
                os: Some(Linux),
            })
        );
    }

    #[test]
    fn test_link_href_to_download4() {
        let download = link_href_to_download("/dl/go1.20.6.darwin-amd64.tar.gz");
        assert_eq!(
            download,
            Some(Download {
                download_url: "https://go.dev/dl/go1.20.6.darwin-amd64.tar.gz".to_string(),
                version: GgVersion::new("1.20.6"),
                tags: HashSet::new(),
                arch: Some(Arch::X86_64),
                variant: Some(Variant::Any),
                os: Some(Mac),
            })
        );
    }

    #[test]
    fn test_link_href_to_download5() {
        let download = link_href_to_download("/dl/go1.19beta1.linux-amd64.tar.gz");
        let tags: HashSet<String> = vec!["beta".to_string()].into_iter().collect();
        assert_eq!(
            download,
            Some(Download {
                download_url: "https://go.dev/dl/go1.19beta1.linux-amd64.tar.gz".to_string(),
                version: GgVersion::new("1.19"),
                tags,
                arch: Some(Arch::X86_64),
                variant: Some(Variant::Any),
                os: Some(Linux),
            })
        );
    }

    #[test]
    fn test_link_href_to_download_extensions() {
        let download = link_href_to_download("/dl/go1.20.6.linux-arm64.tar.gz");
        assert_eq!(download.is_some(), true);
        let download = link_href_to_download("/dl/go1.20.6.linux-arm64.zip");
        assert_eq!(download.is_some(), true);
        let download = link_href_to_download("/dl/go1.20.6.linux-arm64.msi");
        assert_eq!(download.is_some(), false);
        let download = link_href_to_download("/dl/go1.20.6.linux-arm64.pkg");
        assert_eq!(download.is_some(), false);
    }
}
