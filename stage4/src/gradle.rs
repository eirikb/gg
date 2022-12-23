use std::future::Future;
use std::pin::Pin;

use crate::{download_unpack_and_all_that_stuff, Executor};
use crate::target::Target;

use super::target;

pub struct Gradle {}

impl Executor for Gradle {
    fn prep(&self, target: Target) -> Pin<Box<dyn Future<Output=()>>> {
        Box::pin(async move {
            let gradle_url = get_gradle_url(&target).await;
            println!("Gradle download url: {}", gradle_url);
            download_unpack_and_all_that_stuff(&gradle_url, ".cache/gradle").await;
        })
    }

    fn get_bin(&self, target: Target, _: String) -> &str {
        match &target.os {
            target::Os::Windows => "gradle.exe",
            _ => "gradle"
        }
    }

    fn get_path(&self) -> &str {
        "gradle"
    }
}

pub async fn get_gradle_url(_target: &Target) -> String {
    return String::from("https://services.gradle.org/distributions/gradle-6.9.3-bin.zip");
}
