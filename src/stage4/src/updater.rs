use std::env;
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use log::info;

use crate::barus::create_barus;
use crate::bloody_indiana_jones::BloodyIndianaJones;

async fn download_to_temp(temp_path: &str) -> Result<(), String> {
    let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
    let pb = create_barus();

    info!("Downloading to temp file: {}", temp_path);
    let bloody_indiana_jones =
        BloodyIndianaJones::new_with_file_name(url.to_string(), temp_path.to_string(), pb.clone());
    bloody_indiana_jones.download().await;

    // Just in case (FS stuff)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Ok so we need to make temp file executable (Unix only)
    // Good thing we have a temp file, because we can set permissions on it without affecting the original file :D
    #[cfg(unix)]
    {
        if let Ok(mut perms) = fs::metadata(temp_path).and_then(|m| Ok(m.permissions())) {
            perms.set_mode(0o755);
            if let Err(e) = fs::set_permissions(temp_path, perms) {
                return Err(format!("Failed to make temp file executable: {}", e));
            }
            info!("Made temp file executable: {}", temp_path);
        } else {
            return Err("Failed to get temp file permissions".to_string());
        }
    }

    #[cfg(windows)]
    info!("Windows: File should be executable by default");

    Ok(())
}

fn print_version_from_file(file_path: &str) -> Result<String, String> {
    match fs::read(file_path) {
        Ok(bytes) => {
            // Gotta read it special since it is partially binary
            let content = String::from_utf8_lossy(&bytes);

            let new_version =
                if let Some(line) = content.lines().find(|line| line.contains(": VERSION:")) {
                    if let Some(version_str) = line.split(": VERSION:").nth(1) {
                        version_str.trim().to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };

            println!("Successfully updated to version {}!", new_version);
            Ok(new_version)
        }
        Err(e) => Err(format!("Failed to read file: {}", e)),
    }
}

fn execute_version_check(file_path: &str) -> Result<(), String> {
    info!("Testing execution of updated file: {}", file_path);

    let child = std::process::Command::new(file_path)
        .arg("--version")
        .spawn();

    match child {
        Ok(mut process) => {
            let status = process.wait();
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        println!("Update preparation completed!");
                        Ok(())
                    } else {
                        Err(format!(
                            "Update preparation failed with exit code: {:?}",
                            exit_status.code()
                        ))
                    }
                }
                Err(e) => Err(format!("Failed to wait for update preparation: {}", e)),
            }
        }
        Err(e) => Err(format!("Update preparation failed: {}", e)),
    }
}

fn move_temp_to_final(temp_path: &str, final_path: &str) -> Result<(), String> {
    info!(
        "Moving temp file to final location: {} -> {}",
        temp_path, final_path
    );

    if let Err(e) = fs::rename(temp_path, final_path) {
        let _ = fs::remove_file(temp_path);
        return Err(format!("Failed to move temp file: {}", e));
    }

    info!("Atomic move completed successfully");
    Ok(())
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

    let final_path = env::var("GG_CMD_PATH").unwrap_or_else(|_| "gg.cmd".to_string());
    let temp_path = format!("{}.tmp", final_path);

    if let Err(e) = download_to_temp(&temp_path).await {
        println!("Download failed: {}", e);
        return;
    }

    if let Err(e) = print_version_from_file(&temp_path) {
        println!("Failed to read version: {}", e);
        let _ = fs::remove_file(&temp_path);
        return;
    }

    println!("Preparing updated version for faster subsequent updates...");
    if let Err(e) = execute_version_check(&temp_path) {
        println!("Execution test failed: {}", e);
        let _ = fs::remove_file(&temp_path);
        return;
    }

    if let Err(e) = move_temp_to_final(&temp_path, &final_path) {
        println!("Final move failed: {}", e);
        return;
    }
}
