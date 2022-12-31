use std::env;
use std::future::Future;
use std::pin::Pin;
use std::process::Command;

use crate::{Executor, Java};
use crate::executor::{AppInput, Download, prep};

use super::target;

pub struct Gradle {}

impl Executor for Gradle {
    fn get_download_urls(&self, _input: AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>>>> {
        Box::pin(async move {
            vec![
                Download {
                    download_url: "https://services.gradle.org/distributions/gradle-6.9.3-bin.zip".to_string(),
                    version: "".to_string(),
                    lts: false,
                }]
        })
    }

    fn get_bin(&self, input: AppInput) -> &str {
        match &input.target.os {
            target::Os::Windows => "bin/gradle.exe",
            _ => "bin/gradle"
        }
    }

    fn get_name(&self) -> &str {
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

