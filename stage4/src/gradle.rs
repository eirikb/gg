use std::path::PathBuf;

use crate::download_unpack_and_all_that_stuff;
use crate::executor::{prep_bin, try_run};
use crate::target::Target;

use super::target;

async fn prep(target: Target) -> () {
    let gradle_url = get_gradle_url(&target).await;
    println!("Gradle download url: {}", gradle_url);
    download_unpack_and_all_that_stuff(&gradle_url, ".cache/gradle").await;
}

pub async fn prep_gradle(target: Target) -> Result<PathBuf, String> {
    let bin = match &target.os {
        target::Os::Windows => "gradle.exe",
        _ => "gradle"
    };
    prep_bin(bin, "gradle", || Box::pin(prep(target))).await
}

pub async fn try_run_gradle(target: Target) -> Result<(), String> {
    let bin_path = prep_gradle(target).await?.clone();
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

pub async fn get_gradle_url(_target: &Target) -> String {
    return String::from("https://services.gradle.org/distributions/gradle-6.9.3-bin.zip");
}
