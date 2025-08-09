use crate::barus::create_barus;
use crate::executor::{prep, AppInput, GgMeta};
use crate::updater;
use crate::Executor;
use futures_util::future::join_all;
use glob;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use tokio::sync::Semaphore;

struct UpdateInfo {
    tool_name: String,
    version_selector: String,
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
    info!(
        "Checking tool update for cmd: {:?} with version: {:?}",
        meta.cmd.cmd, meta.cmd.version
    );
    if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
        info!(
            "Created executor for: {} (cmd was: {})",
            executor.get_name(),
            meta.cmd.cmd
        );
        let urls = executor.get_download_urls(input).await;
        info!(
            "Got {} urls for {} (cmd: {})",
            urls.len(),
            executor.get_name(),
            meta.cmd.cmd
        );
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

            let version_selector = meta.cmd.to_version_selector();

            return Some(UpdateInfo {
                tool_name: executor.get_name().to_string(),
                version_selector,
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
                match serde_json::from_str::<GgMeta>(&content) {
                    Ok(meta) => {
                        info!(
                            "Successfully parsed meta for: {:?} with version: {:?}",
                            meta.cmd.cmd, meta.cmd.version
                        );
                        metas.push((meta, path));
                    }
                    Err(e) => {
                        info!("Failed to parse meta from {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    info!("Found {} total metas", metas.len());
    metas
}

pub async fn check_or_update_all_including_gg(
    input: &AppInput,
    gg_version: &str,
    should_update: bool,
    allow_major: bool,
    force: bool,
) {
    if should_update {
        updater::perform_update(gg_version, force).await;
    } else {
        updater::check_gg_update(gg_version).await;
    }
    println!();

    check_or_update_all(input, should_update, allow_major, force).await;
}

pub async fn check_or_update_all(
    input: &AppInput,
    should_update: bool,
    allow_major: bool,
    force: bool,
) {
    let metas = get_all_tool_metas().await;

    if metas.is_empty() {
        println!("No cached tools found.");
        return;
    }

    println!("Checking for updates...");

    let m = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix:.bold} {spinner:.green} {msg}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]);

    let semaphore = std::sync::Arc::new(Semaphore::new(5));

    let mut tool_spinners: HashMap<String, ProgressBar> = HashMap::new();

    for (meta, _) in &metas {
        if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
            let tool_name = executor.get_name().to_string();
            let version_selector = meta.cmd.to_version_selector();

            let unique_key = if version_selector.is_empty() {
                tool_name.clone()
            } else {
                format!("{}{}", tool_name, version_selector)
            };

            let pb = m.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix(format!("{:<20}", unique_key));
            pb.set_message("checking...");
            pb.enable_steady_tick(std::time::Duration::from_millis(80));

            tool_spinners.insert(unique_key, pb);
        }
    }

    let check_tasks: Vec<_> = metas
        .into_iter()
        .map(|(meta, path)| {
            let semaphore = semaphore.clone();
            let tool_spinners = tool_spinners.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = check_tool_update(meta, path, input).await;

                if let Some(ref info) = result {
                    let unique_key = if info.version_selector.is_empty() {
                        info.tool_name.clone()
                    } else {
                        format!("{}{}", info.tool_name, info.version_selector)
                    };
                    if let Some(pb) = tool_spinners.get(&unique_key) {
                        pb.finish_with_message("done");
                    }
                }

                result
            }
        })
        .collect();

    let update_infos: Vec<UpdateInfo> = join_all(check_tasks)
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect();

    m.clear().unwrap();

    let filtered_updates: Vec<&UpdateInfo> = if force {
        update_infos.iter().collect()
    } else {
        update_infos
            .iter()
            .filter(|info| should_include_update(info, allow_major))
            .collect()
    };

    println!();

    let mut grouped_tools: HashMap<String, Vec<&UpdateInfo>> = HashMap::new();
    for info in &update_infos {
        grouped_tools
            .entry(info.tool_name.clone())
            .or_insert(Vec::new())
            .push(info);
    }

    for (_tool_name, infos) in grouped_tools {
        for info in infos {
            let current = info.current_version.as_deref().unwrap_or("NA");
            let latest = info.latest_version.as_deref().unwrap_or("NA");
            let status = if force {
                "Will force update"
            } else if !info.needs_update {
                "Up to date"
            } else if info.is_major_update && !allow_major {
                "Major update available (use --major to include)"
            } else {
                "Update available"
            };

            let display_name = if info.version_selector.is_empty() {
                info.tool_name.clone()
            } else {
                format!("{}{}", info.tool_name, info.version_selector)
            };

            println!(
                "{}: Current: {}, Latest: {} - {}",
                display_name, current, latest, status
            );
        }
    }

    if filtered_updates.is_empty() {
        println!("\nAll tools are up to date!");
        return;
    }

    if !should_update {
        println!(
            "\nUpdates available for: {}",
            filtered_updates
                .iter()
                .map(|info| info.tool_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("Run 'update -u' to update all tools, or 'update <tool> -u' for a specific tool.");
        println!("For more options, run 'help'.");
    } else {
        for info in filtered_updates {
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

pub async fn check_or_update_tool(
    input: &AppInput,
    tool_name: &str,
    should_update: bool,
    allow_major: bool,
    force: bool,
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

            let should_perform_update = should_update
                && ((force) || (info.needs_update && (allow_major || !info.is_major_update)));

            if should_perform_update {
                if force {
                    println!("Force updating {}...", tool_name);
                } else {
                    println!("Updating {}...", tool_name);
                }

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
            } else if !info.needs_update {
                println!("{}: Already up to date (version {})", tool_name, current);
            } else if info.is_major_update && !allow_major {
                println!(
                    "{}: Current: {}, Latest: {} - Major update available (use --major to include)",
                    tool_name, current, latest
                );
            } else {
                println!(
                    "{}: Current: {}, Latest: {} - Update available",
                    tool_name, current, latest
                );
                println!("Run 'update {} -u' to update.", tool_name);
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
