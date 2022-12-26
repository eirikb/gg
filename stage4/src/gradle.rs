use std::env;
use std::future::Future;
use std::pin::Pin;
use std::process::Command;

use crate::{download_unpack_and_all_that_stuff, Executor, Java, prep};
use crate::executor::AppInput;
use crate::target::Target;

use super::target;

pub struct Gradle {}

impl Executor for Gradle {
    fn prep(&self, input: AppInput) -> Pin<Box<dyn Future<Output=()>>> {
        Box::pin(async move {
            prep(&Java {}, input.clone()).await.expect("Unable to install Java");
            let gradle_url = get_gradle_url(&input.target).await;
            println!("Gradle download url: {}", gradle_url);
            download_unpack_and_all_that_stuff(&gradle_url, ".cache/gradle").await;
        })
    }

    fn get_bin(&self, input: AppInput) -> &str {
        match &input.target.os {
            target::Os::Windows => "bin/gradle.exe",
            _ => "bin/gradle"
        }
    }

    fn get_path(&self) -> &str {
        "gradle"
    }

    fn before_exec<'a>(&'a self, input: AppInput, command: &'a mut Command) -> Pin<Box<dyn Future<Output=Option<String>> + 'a>> {
        Box::pin(async move {
            let app_path = prep(&Java {}, input.clone()).await.expect("Unable to install Java");
            println!("java path is {:?}", app_path);
            command.env("JAVA_HOME", app_path.app);
            let path_string = &env::var("PATH").unwrap_or("".to_string());
            let bin_path = app_path.bin.to_str().unwrap_or("");
            let path = format!("{bin_path}:{path_string}");
            println!("PATH: {path}");
            Some(path)
        })
    }
}

async fn get_gradle_url(_target: &Target) -> String {
    return String::from("https://services.gradle.org/distributions/gradle-6.9.3-bin.zip");
}
