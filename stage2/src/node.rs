use reqwest::Url;
use scraper::{Html, Selector};

use super::target;

pub async fn get_node_url(target: &target::Target) -> String {
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

fn pick_node_url(target: &target::Target, node_urls: Vec<String>) -> String {
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
    use crate::target::Target;

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
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Windows }, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-win-x64.zip", node_url);
    }

    #[test]
    fn test_pick_node_url_linux_x86() {
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Linux}, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz", node_url);
    }

    #[test]
    fn test_pick_node_url_mac_x86() {
        let node_url = pick_node_url(&Target { arch: target::Arch::X86_64, os: target::Os::Mac}, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-darwin-x64.tar.gz", node_url);
    }

    #[test]
    fn test_pick_node_url_linux_armv7() {
        let node_url = pick_node_url(&Target { arch: target::Arch::Armv7, os: target::Os::Linux}, get_node_urls());
        assert_eq!("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-armv7l.tar.xz", node_url);
    }
}