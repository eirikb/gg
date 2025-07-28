use crate::barus::create_barus;
use crate::executor::{prep, AppInput, GgMeta};
use crate::updater;
use crate::Executor;
use dialoguer::Confirm;
use futures_util::future::join_all;
use glob;
use log::{debug, info};
use std::fs;
use tokio::sync::Semaphore;

struct UpdateInfo {
    tool_name: String,
    current_version: Option<String>,
    latest_version: Option<String>,
    needs_update: bool,
    is_major_update: bool,
    path: std::path::PathBuf,
    executor: Box<dyn Executor>,
}

async fn check_tool_update(
    meta: GgMeta,
    path: std::path::PathBuf,
    input: &AppInput,
) -> Option<UpdateInfo> {
    if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
        let urls = executor.get_download_urls(input).await;
        info!("Got {} urls for {}", urls.len(), executor.get_name());
        let urls_matches = executor.get_url_matches(&urls, input);
        info!(
            "Got {} url matches for {}",
            urls_matches.len(),
            executor.get_name()
        );
        let urls_match = urls_matches.first();
        debug!("Match for {}: {:?}", executor.get_name(), urls_match);

        if let Some(urls_match) = urls_match {
            let current_version = meta.download.version.clone();
            let latest_version = urls_match.version.clone();

            let current_ver = current_version.clone().map(|v| v.to_version());
            let latest_ver = latest_version.clone().map(|v| v.to_version());

            let needs_update = latest_ver > current_ver;
            let is_major_update = if let (Some(current), Some(latest)) = (&current_ver, &latest_ver)
            {
                latest.major > current.major
            } else {
                false
            };

            return Some(UpdateInfo {
                tool_name: executor.get_name().to_string(),
                current_version: current_version.map(|v| v.to_string()),
                latest_version: latest_version.map(|v| v.to_string()),
                needs_update,
                is_major_update,
                path,
                executor,
            });
        }
    }
    None
}

fn should_include_update(update_info: &UpdateInfo, allow_major: bool) -> bool {
    update_info.needs_update && (allow_major || !update_info.is_major_update)
}

async fn get_all_tool_metas() -> Vec<(GgMeta, std::path::PathBuf)> {
    let cache_base_dir = std::env::var("GG_CACHE_DIR").unwrap_or_else(|_| ".cache/gg".to_string());
    let pattern = format!("{cache_base_dir}/**/gg-meta.json");
    let mut metas = Vec::new();

    if let Ok(paths) = glob::glob(&pattern) {
        for path in paths.flatten() {
            info!("Reading meta from {}", path.display());
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(meta) = serde_json::from_str::<GgMeta>(&content) {
                    metas.push((meta, path));
                }
            }
        }
    }
    metas
}

pub async fn check_or_update_all_including_gg(
    input: &AppInput,
    gg_version: &str,
    should_update: bool,
    allow_major: bool,
) {
    println!("Checking for gg updates...");
    if should_update {
        updater::perform_update(gg_version).await;
    } else {
        updater::check_gg_update(gg_version).await;
    }
    println!();

    check_or_update_all(input, should_update, allow_major).await;
}

pub async fn check_or_update_all(input: &AppInput, should_update: bool, allow_major: bool) {
    let metas = get_all_tool_metas().await;

    if metas.is_empty() {
        println!("No cached tools found.");
        return;
    }

    println!("Checking {} tools for updates...", metas.len());

    let semaphore = std::sync::Arc::new(Semaphore::new(5));

    let check_tasks: Vec<_> = metas
        .into_iter()
        .map(|(meta, path)| {
            let semaphore = semaphore.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                check_tool_update(meta, path, input).await
            }
        })
        .collect();

    let update_infos: Vec<UpdateInfo> = join_all(check_tasks)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect();

    let filtered_updates: Vec<&UpdateInfo> = update_infos
        .iter()
        .filter(|info| should_include_update(info, allow_major))
        .collect();

    for info in &update_infos {
        let current = info.current_version.as_deref().unwrap_or("NA");
        let latest = info.latest_version.as_deref().unwrap_or("NA");
        let status = if !info.needs_update {
            "Up to date"
        } else if info.is_major_update && !allow_major {
            "Major update available (use --major to include)"
        } else {
            "Update available"
        };

        println!(
            "{}: Current: {}, Latest: {} - {}",
            info.tool_name, current, latest, status
        );
    }

    if filtered_updates.is_empty() {
        println!("All tools are up to date!");
        return;
    }

    if should_update {
        for info in filtered_updates {
            if Confirm::new()
                .with_prompt(&format!(
                    "Update {} from {} to {}?",
                    info.tool_name,
                    info.current_version.as_deref().unwrap_or("NA"),
                    info.latest_version.as_deref().unwrap_or("NA")
                ))
                .interact()
                .unwrap_or(false)
            {
                println!("Updating {}...", info.tool_name);
                if let Some(parent) = info.path.parent() {
                    if fs::remove_dir_all(parent).is_ok() {
                        let pb = create_barus();
                        let _ = prep(&*info.executor, input, &pb).await;
                        println!("Successfully updated {}", info.tool_name);
                    } else {
                        println!("Unable to update {}", info.tool_name);
                    }
                } else {
                    println!("Unable to update {}", info.tool_name);
                }
            }
        }
    }
}

pub async fn check_or_update_tool(
    input: &AppInput,
    tool_name: &str,
    should_update: bool,
    allow_major: bool,
) {
    let metas = get_all_tool_metas().await;

    let matching_meta = metas.into_iter().find(|(meta, _)| {
        if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
            executor.get_name() == tool_name
        } else {
            false
        }
    });

    if let Some((meta, path)) = matching_meta {
        if let Some(info) = check_tool_update(meta, path, input).await {
            let current = info.current_version.as_deref().unwrap_or("NA");
            let latest = info.latest_version.as_deref().unwrap_or("NA");

            if !info.needs_update {
                println!("{}: Already up to date (version {})", tool_name, current);
            } else if info.is_major_update && !allow_major {
                println!(
                    "{}: Current: {}, Latest: {} - Major update available (use --major to include)",
                    tool_name, current, latest
                );
            } else if should_update {
                if Confirm::new()
                    .with_prompt(&format!(
                        "Update {} from {} to {}?",
                        tool_name, current, latest
                    ))
                    .interact()
                    .unwrap_or(false)
                {
                    println!("Updating {}...", tool_name);
                    if let Some(parent) = info.path.parent() {
                        if fs::remove_dir_all(parent).is_ok() {
                            let pb = create_barus();
                            let _ = prep(&*info.executor, input, &pb).await;
                            println!("Successfully updated {}", tool_name);
                        } else {
                            println!("Unable to update {}", tool_name);
                        }
                    } else {
                        println!("Unable to update {}", tool_name);
                    }
                }
            } else {
                println!(
                    "{}: Current: {}, Latest: {} - Update available! Run 'update {} -u' to update.",
                    tool_name, current, latest, tool_name
                );
            }
        } else {
            println!("Unable to check updates for {}", tool_name);
        }
    } else {
        println!(
            "Tool '{}' not found in cache. Install it first by running: gg {}",
            tool_name, tool_name
        );
    }
}
