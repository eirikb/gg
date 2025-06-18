use std::env;
use std::fs;

use crate::barus::create_barus;
use crate::bloody_indiana_jones::BloodyIndianaJones;

async fn update_download() {
    let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
    let pb = create_barus();
    let file_path = "gg.cmd";
    let bloody_indiana_jones =
        BloodyIndianaJones::new_with_file_name(url.to_string(), file_path.to_string(), pb.clone());
    bloody_indiana_jones.download().await;

    // Just in case
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    match fs::read(file_path) {
        Ok(bytes) => {
            // Gotta read it special since it is partially binary
            let content = String::from_utf8_lossy(&bytes);

            if let Some(line) = content.lines().find(|line| line.contains(": VERSION:")) {
                if let Some(version_str) = line.split(": VERSION:").nth(1) {
                    let new_version = version_str.trim();
                    println!("Successfully updated to version {}!", new_version);
                } else {
                    println!("Update completed!");
                }
            } else {
                println!("Update completed!");
            }
        }
        Err(e) => {
            println!("Failed to read file: {}", e);
            println!("Update completed!");
        }
    }
}

pub async fn perform_update(ver: &str) {
    println!("Checking for updates...");
    println!("Current version: {}", ver);

    let octocrab = octocrab::Octocrab::builder()
        .base_uri("https://ghapi.ggcmd.io/")
        .unwrap()
        .build()
        .expect("Failed to create GitHub API client");

    match octocrab.repos("eirikb", "gg").releases().get_latest().await {
        Ok(release) => {
            let latest_version = release.tag_name.trim_start_matches('v');

            if latest_version == ver {
                println!("Already up to date (version {}).", ver);
                return;
            }

            println!("Updating to version {}...", latest_version);
        }
        Err(_) => {
            println!("Failed to check for updates. Proceeding with download...");
        }
    }

    update_download().await;

    // Just in case :D (wait for FS)
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    println!("Preparing updated version for faster subsequent updates...");
    let gg_cmd_path = env::var("GG_CMD_PATH").unwrap_or_else(|_| "gg.cmd".to_string());
    let child = std::process::Command::new(&gg_cmd_path)
        .arg("--version")
        .spawn();

    match child {
        Ok(mut process) => {
            let status = process.wait();
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        println!("Update preparation completed!");
                    } else {
                        println!(
                            "Update preparation failed with exit code: {:?}",
                            exit_status.code()
                        );
                    }
                }
                Err(e) => println!("Failed to wait for update preparation: {}", e),
            }
        }
        Err(e) => println!("Update completed! (preparation step failed: {})", e),
    }
}
