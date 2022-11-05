use scraper::{Html, Selector};

use super::target;

pub async fn get_gradle_url(target: &target::Target) -> String {
    let body = reqwest::get("https://services.gradle.org/distributions/gradle-6.9.3-bin.zip").await
        .expect("Unable to connect to gradle.org").text().await
        .expect("Unable to download gradle list of versions");

    let url_selector = Selector::parse(".download-matrix a")
        .expect("Unable to find nodejs version to download");
    let document = Html::parse_document(body.as_str());
    let gradle_urls = document.select(&url_selector).map(|x| {
        x.value().attr("href")
            .expect("Unable to find link to gradledownload").to_string()
    }).collect::<Vec<_>>();

    for x in &gradle_urls {
        println!("{}", x);
    }
    let gradle_url = gradle_urls.first().unwrap().to_string();
    println!("URL is {gradle_url}");
    gradle_url
}
