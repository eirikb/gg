use crate::barus::create_barus;
use crate::executor::{prep, AppInput, GgMeta};
use crate::Executor;
use dialoguer::Confirm;
use log::{debug, info};
use std::fs;

pub async fn check(input: &AppInput, update: bool) {
    let cache_base_dir = std::env::var("GG_CACHE_DIR").unwrap_or_else(|_| ".cache/gg".to_string());
    let pattern = format!("{cache_base_dir}/**/gg-meta.json");
    if let Ok(paths) = glob::glob(&pattern) {
        for path in paths.flatten() {
            info!("Checking {}", path.display());
            let meta = serde_json::from_str::<GgMeta>(&fs::read_to_string(&path).unwrap());
            if let Ok(meta) = meta {
                debug!("Meta: {:?}", &meta);
                if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
                    let urls = executor.get_download_urls(input).await;
                    info!("Got {} urls", urls.len());
                    let urls_matches = executor.get_url_matches(&urls, input);
                    info!("Got {} url matches", urls_matches.len());
                    let urls_match = urls_matches.first();
                    debug!("Match: {:?}", urls_match);

                    if let Some(urls_match) = urls_match {
                        let current_version = meta.download.version;
                        let latest_version = &urls_match.version;
                        println!(
                            "{} ({}): Current version: {}. Latest version: {}",
                            executor.get_name(),
                            meta.version_req.to_string(),
                            current_version
                                .clone()
                                .map(|v| v.to_string())
                                .unwrap_or("NA".to_string()),
                            latest_version
                                .clone()
                                .map(|v| v.to_string())
                                .unwrap_or("NA".to_string())
                        );

                        if latest_version.clone().map(|v| v.to_version())
                            > current_version.clone().map(|v| v.to_version())
                        {
                            println!(" ** {}: New version available!", executor.get_name());
                            if update {
                                if Confirm::new()
                                    .with_prompt("Do you want to update?")
                                    .interact()
                                    .unwrap_or(false)
                                {
                                    println!("Updating...");
                                    if let Some(parent) = path.parent() {
                                        if fs::remove_dir_all(parent).is_ok() {
                                            let pb = create_barus();
                                            let e = executor;
                                            let _ = prep(&*e, input, &pb).await;
                                        } else {
                                            println!("Unable to update");
                                        }
                                    } else {
                                        println!("Unable to update");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
