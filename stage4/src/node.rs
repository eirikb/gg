use std::future::Future;
use std::pin::Pin;
use std::process::Command;

use scraper::{Html, Selector};
use serde::Deserialize;
use serde::Serialize;

use crate::download_unpack_and_all_that_stuff;
use crate::executor::{AppInput, Executor};
use crate::target::Target;

use super::target;

type Root = Vec<Root2>;

#[derive(Serialize, Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum LTS {
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Root2 {
    pub version: String,
    pub date: String,
    pub files: Vec<String>,
    pub npm: String,
    pub v8: String,
    pub uv: String,
    pub zlib: String,
    pub openssl: String,
    pub modules: String,
    pub lts: LTS,
    pub security: bool,
}

pub struct Node {}

impl Executor for Node {
    fn prep(&self, input: AppInput) -> Pin<Box<dyn Future<Output=()>>> {
        Box::pin(async move {
            let node_url = get_node_url(&input.target).await;
            println!("Node download url: {}", node_url);
            download_unpack_and_all_that_stuff(&node_url, ".cache/node").await;
        })
    }

    fn get_bin(&self, input: AppInput) -> &str {
        match &input.target.os {
            target::Os::Windows => match input.cmd.as_str() {
                "node" => "node.exe",
                "npm" => "npm.cmd",
                _ => "npx.cmd",
            },
            _ => match input.cmd.as_str() {
                "node" => "bin/node",
                "npm" => "bin/npm",
                _ => "bin/npx"
            }
        }
    }

    fn get_path(&self) -> &str {
        "node"
    }

    fn before_exec(&self, _input: AppInput, _command: &mut Command) -> Pin<Box<dyn Future<Output=Option<String>>>> {
        Box::pin(async { None })
    }
}

async fn get_node_url(target: &Target) -> String {
    match &target.variant {
        target::Variant::Musl => {
            let json = reqwest::get("https://unofficial-builds.nodejs.org/download/release/index.json").await.unwrap().text().await.unwrap();
            let root: Root = serde_json::from_str(json.as_str()).expect("JSON was not well-formatted");
            let r = root.iter().rev().filter(|r|
                match r.lts {
                    LTS::String(_) => true,
                    _ => false
                } && r.files.iter().any(|f| f.contains("musl"))).last().unwrap().clone();
            let version = r.version;
            let file = r.files.iter().find(|f| f.contains("musl")).unwrap();
            String::from(format!("https://unofficial-builds.nodejs.org/download/release/{version}/node-{version}-{file}.tar.xz"))
        }
        _ => {
            let body = reqwest::get("https://nodejs.org/en/download/").await
                .expect("Unable to connect to nodejs.org").text().await
                .expect("Unable to download nodejs list of versions");

            let document = Html::parse_document(body.as_str());
            let url_selector = Selector::parse(".download-matrix a")
                .expect("Unable to find nodejs version to download");

            let node_urls = document.select(&url_selector).map(|x| {
                x.value().attr("href")
                    .expect("Unable to find link to nodejs download").to_string()
            }).collect::<Vec<_>>();

            for x in &node_urls {
                println!("{}", x);
            }
            let node_url = pick_node_url(target, node_urls);
            println!("URL is {node_url}");
            node_url
        }
    }
}

fn pick_node_url(target: &Target, node_urls: Vec<String>) -> String {
    return node_urls.into_iter().filter(|url|
        match &target.arch {
            target::Arch::X86_64 => url.contains("x64"),
            target::Arch::Armv7 => url.contains("armv7l")
        }
    ).find(|url|
        match &target.os {
            target::Os::Linux => url.contains("linux") && url.contains(".tar.xz"),
            target::Os::Windows => url.contains("win") && url.contains(".zip"),
            target::Os::Mac => url.contains("darwin") && url.contains(".tar.gz")
        }
    ).expect("Unable to find matching nodejs version against your arch/os");
}

#[cfg(test)]
mod test {
    use crate::node::pick_node_url;
    use crate::target;
    use crate::target::{Target, Variant};

    fn get_node_urls() -> Vec<String> {
        return vec![
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-x86.msi"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-x64.msi"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-win-x86.zip"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-win-x64.zip"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1.pkg"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-darwin-x64.tar.gz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-darwin-arm64.tar.gz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-armv7l.tar.xz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-arm64.tar.xz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1.tar.gz"),
            String::from("https://hub.docker.com/_/node/"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-ppc64le.tar.xz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-s390x.tar.xz"),
            String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-aix-ppc64.tar.gz"),
        ];
    }

    #[test]
    fn test_pick_node_url_windows_x86() {
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Windows, variant: Variant::None }, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-win-x64.zip", node_url);
    }

    #[test]
    fn test_pick_node_url_linux_x86() {
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Linux, variant: Variant::None }, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz", node_url);
    }

    #[test]
    fn test_pick_node_url_mac_x86() {
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Mac, variant: Variant::None }, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-darwin-x64.tar.gz", node_url);
    }

    #[test]
    fn test_pick_node_url_linux_armv7() {
        let node_url = pick_node_url(&Target { arch: target::Arch::Armv7, os: target::Os::Linux, variant: Variant::None }, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-armv7l.tar.xz", node_url);
    }
}
