use std::env;
use std::fs;

use log::{info, warn};

use crate::barus::create_barus;
use crate::bloody_indiana_jones::BloodyIndianaJones;

async fn download_to_temp(temp_path: &str) -> Result<(), String> {
    let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
    let pb = create_barus();

    info!("Downloading to temp file: {}", temp_path);
    let bloody_indiana_jones =
        BloodyIndianaJones::new_with_file_name(url.to_string(), temp_path.to_string(), pb.clone());
    bloody_indiana_jones.download().await;

    if std::path::Path::new(&bloody_indiana_jones.file_path).exists() {
        fs::copy(&bloody_indiana_jones.file_path, temp_path)
            .map_err(|e| format!("Failed to copy downloaded file: {}", e))?;
    } else {
        return Err("Download failed: file was not downloaded".to_string());
    }

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

            println!("Successfully updated gg to version {}", new_version);
            Ok(new_version)
        }
        Err(e) => Err(format!("Failed to read file: {}", e)),
    }
}

fn execute_version_check(file_path: &str) -> Result<(), String> {
    info!("Testing execution of updated file: {}", file_path);

    // Bah! This is a hacky way to execute the script
    let child = {
        #[cfg(unix)]
        {
            // On Unix, execute through shell since gg.cmd is a shell script
            std::process::Command::new("sh")
                .arg(file_path)
                .arg("--version")
                .spawn()
        }
        #[cfg(windows)]
        {
            if env::var("MSYSTEM").is_ok() || env::var("MINGW_PREFIX").is_ok() {
                std::process::Command::new("sh")
                    .arg(file_path)
                    .arg("--version")
                    .spawn()
            } else if file_path.ends_with(".cmd") || file_path.ends_with(".bat") {
                std::process::Command::new(file_path)
                    .arg("--version")
                    .spawn()
            } else {
                std::process::Command::new("cmd")
                    .arg("/c")
                    .arg(file_path)
                    .arg("--version")
                    .spawn()
            }
        }
    };

    match child {
        Ok(mut process) => {
            let status = process.wait();
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        info!("Setting executable permissions on Unix system");

        match fs::metadata(final_path) {
            Ok(metadata) => {
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);

                if let Err(e) = fs::set_permissions(final_path, permissions) {
                    warn!("Failed to set executable permissions: {}", e);
                } else {
                    info!("Successfully set executable permissions");
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read file metadata for setting permissions: {}",
                    e
                );
            }
        }
    }

    Ok(())
}

pub async fn check_gg_update(ver: &str) {
    let octocrab = octocrab::Octocrab::builder()
        .base_uri("https://ghapi.ggcmd.io/")
        .unwrap()
        .build()
        .expect("Failed to create GitHub API client");

    match octocrab.repos("eirikb", "gg").releases().get_latest().await {
        Ok(release) => {
            let latest_version = release.tag_name.trim_start_matches('v');

            if latest_version == ver {
                println!(
                    "gg: Current: {}, Latest: {} - Up to date",
                    ver, latest_version
                );
            } else {
                println!(
                    "gg: Current: {}, Latest: {} - Update available",
                    ver, latest_version
                );
            }
        }
        Err(_) => {
            println!("gg: Unable to check for updates");
        }
    }
}

pub async fn perform_update(ver: &str, force: bool) {
    let octocrab = octocrab::Octocrab::builder()
        .base_uri("https://ghapi.ggcmd.io/")
        .unwrap()
        .build()
        .expect("Failed to create GitHub API client");

    if !force {
        match octocrab.repos("eirikb", "gg").releases().get_latest().await {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v');

                if latest_version == ver {
                    println!("gg: Already up to date (version {})", ver);
                    return;
                }

                println!("Updating gg to version {}...", latest_version);
            }
            Err(_) => {
                println!("Failed to check for updates. Proceeding with download...");
            }
        }
    } else {
        println!("Force update requested. Proceeding with download...");
    }

    let final_path = env::var("GG_CMD_PATH").unwrap_or_else(|_| "gg.cmd".to_string());
    let final_path = final_path.replace('\\', "/");
    let temp_path = format!("{}.tmp.cmd", final_path);

    println!("Updating: {}", final_path);

    if let Err(e) = download_to_temp(&temp_path).await {
        println!("Download failed: {}", e);
        return;
    }

    match print_version_from_file(&temp_path) {
        Ok(_) => {}
        Err(e) => {
            warn!("Could not read version from downloaded file: {}", e);
            println!("Continuing with update anyway...");
        }
    }

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
