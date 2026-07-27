use crate::barus::create_barus;
use crate::cli::{starts_like_version, strip_version_prefix};
use crate::executor::{prep, AppInput, ExecutorCmd, GgMeta, GgVersionReq};
use crate::tools::{canonical_name, get_all_tools, registry_name};
use crate::updater;
use crate::Executor;
use futures_util::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;
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

fn executor_for(name: &str) -> Option<Box<dyn Executor>> {
    <dyn Executor>::new(ExecutorCmd {
        cmd: name.to_string(),
        version: None,
        distribution: None,
        include_tags: Default::default(),
        exclude_tags: Default::default(),
        gems: None,
    })
}

/// What to tell someone to run when a tool isn't installed, or None when the
/// name isn't a tool at all.
fn install_name(name: &str) -> Option<String> {
    if executor_for(name).is_some() {
        return Some(name.to_string());
    }
    // matches_requested_tool takes repo names off cached tools, so "cli" has
    // to be known here too or it is a typo only while gh is uninstalled.
    // `gg cli` installs nothing though, so name the tool that owns the repo
    get_all_tools()
        .iter()
        .find(|tool| executor_for(tool.name).is_some_and(|e| e.get_name() == name))
        .map(|tool| tool.name.to_string())
}

/// The gg.toml pin for a tool. Exact key first, then the registry name -
/// forward only, like cli.rs. An `npx` key answering a `node` request would
/// have update enforce a pin install never applies.
fn config_pin<'a>(
    dependencies: &'a HashMap<String, String>,
    base_name: &str,
    requested_name: &str,
) -> Option<&'a str> {
    dependencies
        .get(base_name)
        .or_else(|| dependencies.get(requested_name))
        .map(|value| value.as_str())
}

/// Split `node@18` into name and selector. Built-in commands skip the command
/// parser, so update does it itself.
fn split_selector(tool_name: &str) -> (&str, Option<&str>) {
    match tool_name.split_once('@') {
        Some((base, version)) => (base, Some(version)),
        None => (tool_name, None),
    }
}

/// The version out of an `@selector`, `+tag` and `-distribution` stripped the
/// way cli.rs does it. Err carries the part that would not parse.
fn selector_version_req(selector: &str) -> Result<Option<GgVersionReq>, &str> {
    // Tags first - built-in commands skip the tag parser, so java@17+jdk
    // still has them glued on
    let version_part = selector.split('+').next().unwrap_or(selector);
    let version_part = match version_part.split_once('-') {
        // java@-jdk and java@-zulu are distribution only, no version to pin
        // (cli.rs does the same, and README documents java@-jdk+jre)
        Some(("", _)) => return Ok(None),
        // java@17-zulu is version-distribution, but bun@bun-v1.2.0 is all
        // version (#293), so only split when a version comes first
        Some((before, _)) if starts_like_version(before) => before,
        _ => version_part,
    };
    if version_part.is_empty() {
        return Ok(None);
    }
    match GgVersionReq::new(&strip_version_prefix(version_part)) {
        Some(req) => Ok(Some(req)),
        None => Err(version_part),
    }
}

/// Does this cached tool answer to the name the user asked to update?
/// `requested_name` is `tool_name` resolved through the registry.
fn matches_requested_tool(executor: &dyn Executor, tool_name: &str, requested_name: &str) -> bool {
    registry_name(executor) == requested_name
        // The executor name stays accepted so `update <repo-name>` works
        || executor.get_name() == tool_name
        // ...and the exact string it was installed with - the only name a
        // raw gh/owner/repo has
        || executor.get_executor_cmd().cmd == tool_name
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
                tool_name: registry_name(&*executor),
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

    for (meta, path) in &metas {
        if let Some(executor) = <dyn Executor>::new(meta.cmd.clone()) {
            let tool_name = registry_name(&*executor);
            let version_selector = meta.cmd.to_version_selector();

            let label = if version_selector.is_empty() {
                tool_name
            } else {
                format!("{}{}", tool_name, version_selector)
            };

            let pb = m.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix(format!("{:<20}", label));
            pb.set_message("checking...");
            pb.enable_steady_tick(std::time::Duration::from_millis(80));

            // Keyed by cache path, not name: ruby, gem, irb and bundle all
            // resolve to "ruby", and a key collision leaves the loser spinning
            tool_spinners.insert(path.to_string_lossy().to_string(), pb);
        }
    }

    let check_tasks: Vec<_> = metas
        .into_iter()
        .map(|(meta, path)| {
            let semaphore = semaphore.clone();
            let tool_spinners = tool_spinners.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let spinner_key = path.to_string_lossy().to_string();
                let result = check_tool_update(meta, path, input).await;

                if result.is_some() {
                    if let Some(pb) = tool_spinners.get(&spinner_key) {
                        pb.finish_with_message("done");
                    }
                }

                result
            }
        })
        .collect();

    let update_infos: Vec<UpdateInfo> = join_all(check_tasks).await.into_iter().flatten().collect();

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
            .or_default()
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
    config: &crate::config::GgConfig,
) -> ExitCode {
    let metas = get_all_tool_metas().await;

    let (base_name, selector_version) = split_selector(tool_name);
    let requested_name = canonical_name(base_name);

    let config_version = config_pin(&config.dependencies, base_name, &requested_name);

    // An explicit @version beats a gg.toml pin. A selector that will not parse
    // is not the same as no selector - drop it and we check everything, exit 0
    let version_filter = match selector_version {
        Some(selector) => match selector_version_req(selector) {
            Ok(req) => req,
            Err(bad) => {
                eprintln!(
                    "Invalid version '{}' in '{}'. Versions look like @18, @2.95 or @1.2.3",
                    bad, tool_name
                );
                return ExitCode::FAILURE;
            }
        },
        None => config_version.and_then(GgVersionReq::new),
    };

    let name_matches = |meta: &GgMeta| match <dyn Executor>::new(meta.cmd.clone()) {
        Some(executor) => matches_requested_tool(&*executor, base_name, &requested_name),
        None => false,
    };

    let named_metas: Vec<_> = metas
        .into_iter()
        .filter(|(meta, _)| name_matches(meta))
        .collect();
    let named_count = named_metas.len();

    let matching_metas: Vec<_> = named_metas
        .into_iter()
        .filter(
            |(meta, _)| match (&version_filter, &meta.download.version) {
                (Some(req), Some(version)) => req.to_version_req().matches(&version.to_version()),
                _ => true,
            },
        )
        .collect();

    if matching_metas.is_empty() {
        // Not installed is normal, not a failure - callers update before they
        // install (postmortemthis does this per agent), so exit 0 stays.
        // A name that is not a tool at all is a real error though
        return if named_count > 0 {
            // Installed, just not in the version that was asked for
            println!(
                "No cached {} matches {}. Install it by running: gg {}",
                requested_name, tool_name, tool_name
            );
            ExitCode::SUCCESS
        } else if let Some(install) = install_name(base_name) {
            println!(
                "{} is not installed yet, nothing to update. Install it by running: gg {}",
                tool_name, install
            );
            ExitCode::SUCCESS
        } else {
            eprintln!(
                "Unknown tool '{}'. Run 'gg tools' to see the available tools.",
                tool_name
            );
            ExitCode::FAILURE
        };
    }

    let mut update_available = false;
    let mut update_failed = false;

    for (meta, path) in matching_metas {
        if let Some(info) = check_tool_update(meta, path, input).await {
            let display_name = if info.version_selector.is_empty() {
                info.tool_name.clone()
            } else {
                format!("{}{}", info.tool_name, info.version_selector)
            };
            let current = info.current_version.as_deref().unwrap_or("NA");
            let latest = info.latest_version.as_deref().unwrap_or("NA");

            let should_perform_update = should_update
                && ((force) || (info.needs_update && (allow_major || !info.is_major_update)));

            if should_perform_update {
                if force {
                    println!("Force updating {}...", display_name);
                } else {
                    println!("Updating {}...", display_name);
                }

                if let Some(parent) = info.path.parent() {
                    if fs::remove_dir_all(parent).is_ok() {
                        let pb = create_barus();
                        // Cache dir is already gone, so a failed prep leaves
                        // the tool uninstalled - "Successfully updated" and
                        // exit 0 there is how you lose a tool quietly
                        match prep(&*info.executor, input, &pb).await {
                            Ok(_) => println!("Successfully updated {}", display_name),
                            Err(e) => {
                                eprintln!("Failed to update {}: {}", display_name, e);
                                update_failed = true;
                            }
                        }
                    } else {
                        eprintln!("Unable to update {}", display_name);
                        update_failed = true;
                    }
                } else {
                    eprintln!("Unable to update {}", display_name);
                    update_failed = true;
                }
            } else if !info.needs_update {
                println!("{}: Already up to date (version {})", display_name, current);
            } else if info.is_major_update && !allow_major {
                println!(
                    "{}: Current: {}, Latest: {} - Major update available (use --major to include)",
                    display_name, current, latest
                );
            } else {
                println!(
                    "{}: Current: {}, Latest: {} - Update available",
                    display_name, current, latest
                );
                update_available = true;
            }
        } else {
            println!("Unable to check updates for {}", tool_name);
        }
    }

    if update_available {
        println!("Run 'update {} -u' to update.", tool_name);
    }

    if update_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{Download, ExecutorCmd, GgVersionReq};
    use std::collections::HashSet;

    fn meta_for(cmd: &str) -> GgMeta {
        GgMeta {
            version_req: GgVersionReq::new("*").unwrap(),
            download: Download::new("https://example.com/x.tar.gz".to_string(), "1.0.0", None),
            cmd: ExecutorCmd {
                cmd: cmd.to_string(),
                version: None,
                distribution: None,
                include_tags: HashSet::new(),
                exclude_tags: HashSet::new(),
                gems: None,
            },
        }
    }

    fn name_for(cmd: &str) -> String {
        let executor = <dyn Executor>::new(meta_for(cmd).cmd).unwrap();
        registry_name(&*executor)
    }

    #[test]
    fn test_registry_name_uses_tool_name_not_repo() {
        // Repo differs from the tool name, so `update <tool>` used to miss
        assert_eq!(name_for("gh"), "gh"); // repo cli/cli
        assert_eq!(name_for("git"), "git"); // repo eirikb/portable-git
        assert_eq!(name_for("antigravity"), "antigravity"); // repo antigravity-cli
    }

    #[test]
    fn test_registry_name_folds_aliases() {
        assert_eq!(name_for("claude-code"), "claude");
        assert_eq!(name_for("antigravity-cli"), "antigravity");
        assert_eq!(name_for("agy"), "antigravity");
        assert_eq!(name_for("npx"), "node");
    }

    #[test]
    fn test_registry_name_falls_back_for_unregistered() {
        // A raw gh/owner/repo is not in the registry - keep the executor name
        assert_eq!(
            name_for("gh/google-antigravity/antigravity-cli"),
            "antigravity-cli"
        );
    }

    /// `cached` is what the cache was installed as, `asked` what gets typed
    /// at `gg update <asked>`
    fn matches(cached: &str, asked: &str) -> bool {
        let executor = <dyn Executor>::new(meta_for(cached).cmd).unwrap();
        matches_requested_tool(&*executor, asked, &canonical_name(asked))
    }

    #[test]
    fn test_matches_primary_name_when_repo_differs() {
        assert!(matches("gh", "gh"));
        assert!(matches("git", "git"));
        assert!(matches("antigravity", "antigravity"));
    }

    #[test]
    fn test_matches_aliases_either_way() {
        assert!(matches("claude", "claude-code"));
        assert!(matches("claude-code", "claude"));
        assert!(matches("node", "npx"));
        assert!(matches("antigravity", "agy"));
    }

    #[test]
    fn test_matches_repo_name_still_accepted() {
        // These worked before the fix by accident, keep them working
        assert!(matches("gh", "cli"));
        assert!(matches("git", "portable-git"));
    }

    #[test]
    fn test_install_name_separates_typos_from_not_installed() {
        // Real tools that may not be installed - these have to stay exit 0
        for name in [
            "vibe",
            "gh",
            "claude-code",
            "antigravity-cli",
            "gh/google-antigravity/antigravity-cli",
        ] {
            assert_eq!(install_name(name).as_deref(), Some(name));
        }

        // Repo names too, and `gg cli` installs nothing, so point at the tool
        assert_eq!(install_name("cli").as_deref(), Some("gh"));
        assert_eq!(install_name("portable-git").as_deref(), Some("git"));

        // Not tools at all - these are the ones worth failing on
        for name in ["totally-bogus-tool", "", "gh/no-repo-part"] {
            assert_eq!(install_name(name), None, "{name}");
        }
    }

    /// A still-attached selector must not read as a typo
    #[test]
    fn test_version_selector_is_split_off_the_name() {
        for (input, base, version) in [
            ("node@18", "node", Some("18")),
            ("gh@2.95", "gh", Some("2.95")),
            ("java@17", "java", Some("17")),
            ("java@-jdk+jre", "java", Some("-jdk+jre")),
            // First @ wins, same as cli.rs taking parts[0] of split("@")
            ("node@18@extra", "node", Some("18@extra")),
            ("claude-code", "claude-code", None),
            (
                "gh/google-antigravity/antigravity-cli",
                "gh/google-antigravity/antigravity-cli",
                None,
            ),
        ] {
            assert_eq!(split_selector(input), (base, version), "splitting {input}");
            assert!(
                install_name(base).is_some(),
                "{input} must not read as unknown"
            );
        }
    }

    #[test]
    fn test_selector_version_req() {
        let ok = |s: &str| selector_version_req(s).unwrap().map(|r| r.to_version_req());

        // Has to be an error, not "no filter" - that checked everything, exit 0
        assert_eq!(selector_version_req("bogus").err(), Some("bogus"));
        assert_eq!(selector_version_req("lts").err(), Some("lts"));

        // Distribution with no version pins nothing (README has java@-jdk+jre)
        assert!(ok("-jdk").is_none());
        assert!(ok("-zulu").is_none());
        assert!(ok("-jdk+jre").is_none());

        // Tags and distribution come off, the version survives
        assert!(ok("18").unwrap().matches(&"18.20.8".parse().unwrap()));
        assert!(ok("17+jdk").unwrap().matches(&"17.0.1".parse().unwrap()));
        assert!(ok("17-zulu").unwrap().matches(&"17.0.1".parse().unwrap()));
        // bun@bun-v1.2.0 is all version, not version-distribution (#293)
        assert!(ok("bun-v1.2.0").unwrap().matches(&"1.2.0".parse().unwrap()));

        // Nothing to pin is fine, that is not the same as unparseable
        assert!(ok("").is_none());
        assert!(ok("+jdk").is_none());
    }

    fn pins(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_config_pin_prefers_exact_key() {
        let deps = pins(&[("node", "18"), ("npx", "20")]);

        // Both keys resolve to "node", so without exact-key-first this picks
        // whichever the HashMap felt like today
        assert_eq!(config_pin(&deps, "node", "node"), Some("18"));
        assert_eq!(config_pin(&deps, "npx", "node"), Some("20"));

        // Forward only, the direction cli.rs resolves an install with
        let only_node = pins(&[("node", "18")]);
        assert_eq!(config_pin(&only_node, "npx", "node"), Some("18"));

        // ...and not the reverse, or update pins what install would not
        let only_npx = pins(&[("npx", "20")]);
        assert_eq!(config_pin(&only_npx, "node", "node"), None);

        assert_eq!(config_pin(&only_node, "deno", "deno"), None);
    }

    /// Puts GG_CACHE_DIR back on the way out, panic or not, or a failed assert
    /// leaves the rest of the binary pointed at a deleted dir
    struct CacheDirGuard(Option<String>);

    impl Drop for CacheDirGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(previous) => std::env::set_var("GG_CACHE_DIR", previous),
                None => std::env::remove_var("GG_CACHE_DIR"),
            }
        }
    }

    /// One test for every cache-dependent assertion: GG_CACHE_DIR is process
    /// global and tests run in parallel, so splitting these would race.
    #[tokio::test]
    async fn test_update_exit_codes() {
        let _guard = CacheDirGuard(std::env::var("GG_CACHE_DIR").ok());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("GG_CACHE_DIR", dir.path());

        // Without that set_var this globs the real cache and the asserts below
        // go online, so fail here instead
        assert!(get_all_tool_metas().await.is_empty());

        let input = AppInput {
            target: crate::target::Target::parse_with_overrides(
                "x86_64-unknown-linux-gnu",
                None,
                None,
            ),
            app_args: vec![],
        };
        let config = crate::config::GgConfig {
            dependencies: HashMap::new(),
            aliases: HashMap::new(),
        };
        let run = |name: &'static str| {
            let input = &input;
            let config = &config;
            async move { check_or_update_tool(input, name, false, false, false, config).await }
        };

        // Not installed is normal - a nonzero here breaks callers
        assert_eq!(run("vibe").await, ExitCode::SUCCESS);
        assert_eq!(run("gh").await, ExitCode::SUCCESS);
        // Repo name, so it must not flip to "unknown" on an empty cache
        assert_eq!(run("cli").await, ExitCode::SUCCESS);
        // Selector still attached, must not read as a typo
        assert_eq!(run("node@18").await, ExitCode::SUCCESS);

        // Not a tool at all, and a version that will not parse
        assert_eq!(run("totally-bogus-tool").await, ExitCode::FAILURE);
        assert_eq!(run("gh@bogus").await, ExitCode::FAILURE);
    }

    #[test]
    fn test_matches_raw_github_by_exact_name() {
        // The only name a raw gh/owner/repo answers to
        let raw = "gh/google-antigravity/antigravity-cli";
        assert!(matches(raw, raw));
        // ...and it must not swallow the registry tool with the same repo
        assert!(!matches("antigravity", raw));
    }

    #[test]
    fn test_does_not_match_other_tools() {
        assert!(!matches("gh", "git"));
        assert!(!matches("claude", "codex"));
        assert!(!matches("node", "deno"));
        assert!(!matches("antigravity", "gemini-cli"));
    }
}
