use crate::config::GgConfig;
use crate::executor::find_version;
use crate::tools::get_tool_info;
use clap::{ArgAction, Parser, Subcommand};
use regex::{Match, Regex};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ClapCmd {
    pub cmd: String,
    pub version: Option<String>,
    pub distribution: Option<String>,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
    pub gems: Option<Vec<String>>,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "gg",
    about = "A tool manager that downloads and executes tools on demand",
    long_about = None,
    version,
    disable_help_flag = true,
    disable_version_flag = true,
    allow_external_subcommands = true,
    trailing_var_arg = true
)]
pub struct Cli {
    #[arg(
        short = 'l',
        help = "Use local cache (.cache/gg) instead of global cache"
    )]
    pub local_cache: bool,

    #[arg(short = 'v', action = ArgAction::Count, help = "Increase verbosity level")]
    pub verbosity: u8,

    #[arg(short = 'w', help = "Even more output")]
    pub log_external: bool,

    #[arg(short = 'h', long = "help", help = "Print help")]
    pub help: bool,

    #[arg(short = 'V', long = "version", help = "Print version")]
    pub version: bool,

    #[arg(long = "os", help = "Override target OS (windows, linux, mac)")]
    pub override_os: Option<String>,

    #[arg(
        long = "arch",
        help = "Override target architecture (x86_64, arm64, armv7)"
    )]
    pub override_arch: Option<String>,

    #[arg(short = 'u', help = "Actually perform the update (vs just checking)")]
    pub update_flag: bool,

    #[arg(
        long = "major",
        help = "Include major version updates (default: skip major versions)"
    )]
    pub major_flag: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

/// Applet dispatch (busybox-style) with a jump-out, applied to raw argv:
/// - A literal `gg` first argument bypasses applet dispatch and is stripped,
///   so a renamed gg.cmd still works as plain gg: `node.cmd gg update` behaves
///   as `gg.cmd update`. This lets a tool launched via its own applet call
///   back into gg through the same .cmd file.
/// - Otherwise, if gg.cmd has been renamed, the new name is prepended as the
///   command: `postmortemthis.cmd args` behaves as `gg.cmd postmortemthis args`.
pub fn apply_applet_dispatch(raw_args: &mut Vec<String>, cmd_path: Option<&str>) {
    if raw_args.get(1).map(String::as_str) == Some("gg") {
        raw_args.remove(1);
    } else if let Some(applet) = cmd_path.and_then(applet_from_cmd_path) {
        raw_args.insert(1, applet);
    }
}

/// If gg.cmd has been renamed (e.g. to `postmortemthis.cmd`), the new name
/// acts as an applet (busybox-style): `postmortemthis.cmd args` behaves as
/// `gg.cmd postmortemthis args`. The name is resolved like any other command
/// (tool registry or gg.toml alias).
///
/// The wrapper (`.cmd`) and the self-updater's temp marker (`.tmp`, from the
/// `<name>.tmp.cmd` file it runs `--version` against before swapping it in) are
/// stripped before matching, so `gg`, `gg.cmd` and `gg.tmp.cmd` all resolve to
/// gg itself rather than a phantom applet. Internal dots survive, so a real
/// applet like `my.alias.cmd` is still detected.
pub fn applet_from_cmd_path(path: &str) -> Option<String> {
    let base = path.rsplit(['/', '\\']).next()?;
    let mut name = base;
    while let Some(trimmed) = name
        .strip_suffix(".cmd")
        .or_else(|| name.strip_suffix(".CMD"))
        .or_else(|| name.strip_suffix(".tmp"))
        .or_else(|| name.strip_suffix(".TMP"))
    {
        name = trimmed;
    }
    if name.is_empty() || name.eq_ignore_ascii_case("gg") {
        None
    } else {
        Some(name.to_string())
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(about = "Check for updates for all tools (including gg)")]
    Update {
        #[arg(help = "Tool name to update (e.g., flutter, gg)")]
        tool: Option<String>,
        #[arg(short = 'u', help = "Actually perform the update (vs just checking)")]
        update: bool,
        #[arg(long = "major", help = "Include major version updates")]
        major: bool,
        #[arg(
            short = 'f',
            long = "force",
            help = "Force re-download even if already up to date (requires -u)"
        )]
        force: bool,
    },
    #[command(about = "List all available tools")]
    Tools {
        #[arg(help = "Tool name to get info about")]
        tool: Option<String>,
    },
    #[command(name = "clean-cache", about = "Clean cache (prompts for confirmation)")]
    CleanCache,
    #[command(about = "Manage gg configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    #[command(about = "Initialize a new gg.toml config file")]
    Init,
    #[command(about = "Show current configuration")]
    Show,
}

impl Cli {
    pub fn parse_args(&self, config: &GgConfig) -> (Vec<ClapCmd>, Vec<String>) {
        if let Some(command) = &self.command {
            match command {
                Commands::Update { tool, .. } => {
                    let app_args = tool.as_ref().map(|t| vec![t.clone()]).unwrap_or_default();
                    (
                        vec![ClapCmd {
                            cmd: "update".to_string(),
                            version: None,
                            distribution: None,
                            include_tags: HashSet::new(),
                            exclude_tags: HashSet::new(),
                            gems: None,
                        }],
                        app_args,
                    )
                }
                Commands::Tools { tool } => {
                    let app_args = tool.as_ref().map(|t| vec![t.clone()]).unwrap_or_default();
                    (
                        vec![ClapCmd {
                            cmd: "tools".to_string(),
                            version: None,
                            distribution: None,
                            include_tags: HashSet::new(),
                            exclude_tags: HashSet::new(),
                            gems: None,
                        }],
                        app_args,
                    )
                }
                Commands::CleanCache => (
                    vec![ClapCmd {
                        cmd: "clean-cache".to_string(),
                        version: None,
                        distribution: None,
                        include_tags: HashSet::new(),
                        exclude_tags: HashSet::new(),
                        gems: None,
                    }],
                    vec![],
                ),
                Commands::Config { action } => {
                    let cmd = match action {
                        ConfigAction::Init => "config-init",
                        ConfigAction::Show => "config-show",
                    };
                    (
                        vec![ClapCmd {
                            cmd: cmd.to_string(),
                            version: None,
                            distribution: None,
                            include_tags: HashSet::new(),
                            exclude_tags: HashSet::new(),
                            gems: None,
                        }],
                        vec![],
                    )
                }
            }
        } else if let Some(first_arg) = self.args.first() {
            if let Some(alias_commands) = config.resolve_alias_with_and(first_arg) {
                if alias_commands.len() > 1 {
                    return (
                        vec![ClapCmd {
                            cmd: format!("__multi_alias__{}", first_arg),
                            version: None,
                            distribution: None,
                            include_tags: HashSet::new(),
                            exclude_tags: HashSet::new(),
                            gems: None,
                        }],
                        self.args[1..].to_vec(),
                    );
                }
            }

            if let Some(alias_args) = config.resolve_alias(first_arg) {
                let mut expanded_args = alias_args;
                expanded_args.extend_from_slice(&self.args[1..]);

                let mut depth = 0;
                const MAX_DEPTH: usize = 10;
                while depth < MAX_DEPTH {
                    if let Some(first) = expanded_args.first() {
                        if let Some(nested_alias) = config.resolve_alias(first) {
                            let rest = expanded_args[1..].to_vec();
                            expanded_args = nested_alias;
                            expanded_args.extend(rest);
                            depth += 1;
                            continue;
                        }
                    }
                    break;
                }

                let cmds = if let Some(cmd_part) = expanded_args.first() {
                    parse_command_string(cmd_part, config)
                } else {
                    vec![]
                };
                let app_args = expanded_args[1..].to_vec();
                (cmds, app_args)
            } else {
                let cmds = parse_command_string(first_arg, config);
                let app_args = self.args[1..].to_vec();

                if cmds.len() == 1
                    && cmds[0].cmd == "run"
                    && app_args.first().is_some_and(|a| {
                        a.starts_with("gh/") || get_tool_info(a).is_some()
                    })
                {
                    let tool = &app_args[0];
                    let new_cmds = parse_command_string(tool, config);
                    let new_app_args = app_args[1..].to_vec();
                    return (new_cmds, new_app_args);
                }

                (cmds, app_args)
            }
        } else {
            (vec![], vec![])
        }
    }

    pub fn get_log_level(&self) -> String {
        match self.verbosity {
            0 => "warn".to_string(),
            1 => "info".to_string(),
            2 => "debug".to_string(),
            _ => "trace".to_string(),
        }
    }

    pub fn get_update_flag(&self) -> bool {
        if let Some(Commands::Update { update, .. }) = &self.command {
            *update
        } else {
            self.update_flag
        }
    }

    pub fn get_major_flag(&self) -> bool {
        if let Some(Commands::Update { major, .. }) = &self.command {
            *major
        } else {
            self.major_flag
        }
    }

    pub fn get_force_flag(&self) -> bool {
        if let Some(Commands::Update { force, .. }) = &self.command {
            *force
        } else {
            false
        }
    }
}

/// Strip a product or `v` prefix off a version, keeping any leading range
/// operator and partial form: "bun-v1.2.0" -> "1.2.0", "=v1.2.0" -> "=1.2.0",
/// "^18" -> "^18". Left untouched if there's no version in it.
fn strip_version_prefix(version: &str) -> String {
    let op_end = version
        .find(|c: char| !matches!(c, '^' | '~' | '=' | '<' | '>'))
        .unwrap_or(version.len());
    let (op, rest) = version.split_at(op_end);
    match find_version(rest) {
        Some(v) => format!("{op}{v}"),
        None => version.to_string(),
    }
}

/// True when `s` starts like a version (optional range operator, optional `v`,
/// then a digit), so it's the version half of `version-distribution` and not a
/// product prefix like "bun". The operator has to be skipped too, or
/// "^17-temurin" reads as a raw tag and loses the distribution.
fn starts_like_version(s: &str) -> bool {
    let s = s.trim_start_matches(['^', '~', '=', '<', '>']);
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    s.starts_with(|c: char| c.is_ascii_digit())
}

fn parse_command_string(cmd_string: &str, config: &GgConfig) -> Vec<ClapCmd> {
    cmd_string
        .split(":")
        .filter(|s| !s.is_empty())
        .map(|cmd| {
            let mut cmd = cmd.to_string();
            let parts: Vec<String> = cmd.split("@").map(String::from).collect();
            let mut include_tags = HashSet::new();
            let mut exclude_tags = HashSet::new();
            let mut version = None;
            let mut distribution = None;

            let base_cmd = if parts.len() >= 2 {
                cmd = parts[0].to_string();
                parts[0].clone()
            } else {
                cmd.clone()
            };

            if parts.len() == 2 {
                let alles = parts[1].to_string();
                let mut version_dist_end = alles.len();
                let mut chars = alles.char_indices().peekable();
                let in_version = true;
                let mut found_dash_in_version = false;

                while let Some((i, ch)) = chars.next() {
                    match ch {
                        '-' if in_version => {
                            if let Some((_, next_ch)) = chars.peek() {
                                if next_ch.is_alphabetic() && !found_dash_in_version {
                                    found_dash_in_version = true;
                                    continue;
                                }
                            }
                            version_dist_end = i;
                            break;
                        }
                        '+' => {
                            version_dist_end = i;
                            break;
                        }
                        '-' if !in_version => {
                            version_dist_end = i;
                            break;
                        }
                        _ => {}
                    }
                }

                let version_dist_part = alles[0..version_dist_end].to_string();

                if let Some(dash_pos) = version_dist_part.find('-') {
                    let before = version_dist_part[0..dash_pos].to_string();
                    let after = version_dist_part[dash_pos + 1..].to_string();
                    if !before.is_empty() && !starts_like_version(&before) {
                        // A raw release tag like "bun-v1.2.0", not
                        // "version-distribution" - the whole thing is the
                        // version, so strip the prefix and don't pin "bun" (#293)
                        version = Some(strip_version_prefix(&version_dist_part));
                    } else {
                        version = if before.is_empty() {
                            None
                        } else {
                            Some(strip_version_prefix(&before))
                        };
                        distribution = Some(after);
                    }
                } else if !version_dist_part.is_empty() {
                    version = Some(strip_version_prefix(&version_dist_part));
                }

                if version_dist_end < alles.len() {
                    let tag_regex = Regex::new(r"[+-]").unwrap();
                    let tag_part = &alles[version_dist_end..];
                    let tag_matches = tag_regex.find_iter(tag_part).collect::<Vec<Match>>();

                    tag_matches.iter().enumerate().for_each(|(index, m)| {
                        let until = if index < tag_matches.len() - 1 {
                            tag_matches[index + 1].start()
                        } else {
                            tag_part.len()
                        };
                        let command = tag_part[m.start()..m.start() + 1].to_string();
                        let text = tag_part[m.start() + 1..until].to_string();
                        if command == "+" {
                            include_tags.insert(text);
                        } else if command == "-" {
                            exclude_tags.insert(text);
                        }
                    });
                }
            }

            if version.is_none() {
                if let Some(dep_version) = config.dependencies.get(&base_cmd) {
                    version = Some(dep_version.clone());
                } else {
                    // Piggybacking!
                    let underlying_tool = match base_cmd.as_str() {
                        "npm" | "npx" => "node",
                        "dart" => "flutter",
                        _ => &base_cmd,
                    };
                    if let Some(dep_version) = config.dependencies.get(underlying_tool) {
                        version = Some(dep_version.clone());
                    }
                }
            }

            ClapCmd {
                cmd,
                version,
                distribution,
                include_tags,
                exclude_tags,
                gems: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_args(args: Vec<&str>) -> Cli {
        let args = std::iter::once("gg")
            .chain(args.into_iter())
            .map(String::from)
            .collect::<Vec<_>>();
        Cli::try_parse_from(args).unwrap()
    }

    #[test]
    fn test_applet_from_cmd_path() {
        assert_eq!(applet_from_cmd_path("./gg.cmd"), None);
        assert_eq!(applet_from_cmd_path("/home/x/gg.cmd"), None);
        assert_eq!(applet_from_cmd_path("C:\\tools\\GG.CMD"), None);
        assert_eq!(applet_from_cmd_path("gg"), None);
        assert_eq!(applet_from_cmd_path(""), None);
        assert_eq!(
            applet_from_cmd_path("./postmortemthis.cmd"),
            Some("postmortemthis".to_string())
        );
        assert_eq!(
            applet_from_cmd_path("/a/b/build.cmd"),
            Some("build".to_string())
        );
        assert_eq!(
            applet_from_cmd_path("C:\\proj\\deploy.cmd"),
            Some("deploy".to_string())
        );
        assert_eq!(applet_from_cmd_path("node"), Some("node".to_string()));
        assert_eq!(
            applet_from_cmd_path("my.alias.cmd"),
            Some("my.alias".to_string())
        );
    }

    #[test]
    fn test_self_update_temp_is_not_an_applet() {
        // The updater downloads to `<name>.tmp.cmd` and runs `--version` against
        // it. That file must be recognized as gg, not parsed as an applet named
        // after the temp file, regardless of whether gg.cmd was named `gg` or
        // `gg.cmd` and regardless of its path.
        for temp in [
            "gg.tmp.cmd",
            "gg.cmd.tmp.cmd",
            "/home/x/bin/gg.tmp.cmd",
            "/usr/local/bin/gg.cmd.tmp.cmd",
            "./gg.cmd.tmp.cmd",
            ".cache/gg/gg.tmp.cmd",
            "C:\\tools\\GG.TMP.CMD",
        ] {
            assert_eq!(applet_from_cmd_path(temp), None, "{} should be gg", temp);
        }
    }

    #[test]
    fn test_self_update_version_check_contract() {
        // Frozen contract: an already-installed (old) gg runs the new binary as
        // `sh <name>.tmp.cmd --version` to verify it. That invocation must parse
        // as the version flag, never as an applet. Locks the self-update path so
        // a future argv feature can't silently break it again.
        for temp in ["gg.tmp.cmd", "gg.cmd.tmp.cmd", "/home/x/bin/gg.tmp.cmd"] {
            let mut raw_args = vec!["gg".to_string(), "--version".to_string()];
            apply_applet_dispatch(&mut raw_args, Some(temp));
            let cli = Cli::try_parse_from(&raw_args).unwrap();
            assert!(
                cli.version,
                "`--version` via {} must set the version flag",
                temp
            );
        }
    }

    #[test]
    fn test_self_update_literal_gg_arg_bypasses_applet() {
        // Layer 2: the updater passes a literal `gg` first arg so the jump-out
        // bypasses applet dispatch outright, even for an applet-named gg.cmd.
        let mut raw_args = vec!["gg".to_string(), "gg".to_string(), "--version".to_string()];
        apply_applet_dispatch(&mut raw_args, Some("./node.cmd"));
        assert_eq!(raw_args, vec!["gg", "--version"]);
        let cli = Cli::try_parse_from(&raw_args).unwrap();
        assert!(cli.version);
    }

    fn dispatch(args: Vec<&str>, cmd_path: Option<&str>) -> Vec<String> {
        let mut raw_args: Vec<String> = args.into_iter().map(String::from).collect();
        apply_applet_dispatch(&mut raw_args, cmd_path);
        raw_args
    }

    #[test]
    fn test_applet_dispatch_prepends_renamed_cmd() {
        assert_eq!(
            dispatch(vec!["gg", "--version"], Some("./node.cmd")),
            vec!["gg", "node", "--version"]
        );
    }

    #[test]
    fn test_applet_dispatch_plain_gg_cmd_unchanged() {
        assert_eq!(
            dispatch(vec!["gg", "node", "hello"], Some("./gg.cmd")),
            vec!["gg", "node", "hello"]
        );
        assert_eq!(dispatch(vec!["gg", "node"], None), vec!["gg", "node"]);
    }

    #[test]
    fn test_applet_dispatch_gg_jumpout() {
        // `node.cmd gg update` behaves as `gg.cmd update`
        assert_eq!(
            dispatch(vec!["gg", "gg", "update"], Some("./node.cmd")),
            vec!["gg", "update"]
        );
        // Uniform without an applet too: `gg.cmd gg node` = `gg.cmd node`
        assert_eq!(
            dispatch(vec!["gg", "gg", "node"], Some("./gg.cmd")),
            vec!["gg", "node"]
        );
        // Strictly lowercase `gg` is the jump-out word
        assert_eq!(
            dispatch(vec!["gg", "GG", "update"], Some("./node.cmd")),
            vec!["gg", "node", "GG", "update"]
        );
    }

    #[test]
    fn test_node_with_args() {
        let cli = parse_test_args(vec!["node", "hello", "world"]);
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(app_args, vec!["hello", "world"]);
    }

    #[test]
    fn test_version_flag() {
        let cli = parse_test_args(vec!["-V", "node", "hello", "world"]);
        assert!(cli.version);
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(app_args, vec!["hello", "world"]);
    }

    #[test]
    fn test_verbosity_levels() {
        let cli1 = parse_test_args(vec!["-v", "node"]);
        assert_eq!(cli1.get_log_level(), "info");

        let cli2 = parse_test_args(vec!["-vv", "node"]);
        assert_eq!(cli2.get_log_level(), "debug");

        let cli3 = parse_test_args(vec!["-vvv", "node"]);
        assert_eq!(cli3.get_log_level(), "trace");
    }

    #[test]
    fn test_update_command() {
        let cli = parse_test_args(vec!["update"]);
        let config = GgConfig::default();
        let (cmds, _) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "update");
    }

    #[test]
    fn test_versioning() {
        let cli = parse_test_args(vec!["node@10:gradle@1.2.3", "hello", "world"]);
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);

        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "10");

        assert_eq!(cmds[1].cmd, "gradle");
        assert_eq!(cmds[1].version.as_ref().unwrap(), "1.2.3");

        assert_eq!(app_args, vec!["hello", "world"]);
    }

    #[test]
    fn test_os_arch_overrides() {
        let cli = parse_test_args(vec![
            "--os",
            "windows",
            "--arch",
            "arm64",
            "-v",
            "deno",
            "--version",
        ]);
        assert_eq!(cli.override_os.as_ref().unwrap(), "windows");
        assert_eq!(cli.override_arch.as_ref().unwrap(), "arm64");
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "deno");
        assert_eq!(app_args, vec!["--version"]);
    }

    #[test]
    fn test_distribution_parsing() {
        let cli = parse_test_args(vec!["java@17-temurin", "hello"]);
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "java");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "17");
        assert_eq!(cmds[0].distribution.as_ref().unwrap(), "temurin");
        assert_eq!(app_args, vec!["hello"]);
    }

    #[test]
    fn test_distribution_with_tags() {
        let cli = parse_test_args(vec!["java@17-temurin+lts-ea", "hello"]);
        let config = GgConfig::default();
        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "java");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "17");
        assert_eq!(cmds[0].distribution.as_ref().unwrap(), "temurin");
        assert!(cmds[0].include_tags.contains("lts"));
        assert!(cmds[0].exclude_tags.contains("ea"));
        assert_eq!(app_args, vec!["hello"]);
    }

    #[test]
    fn test_github_full_tag_version_pinning() {
        // Pasting bun's full tag must pin the version, not read "bun" as the
        // version and fall back to latest (#293)
        let cli = parse_test_args(vec!["gh/oven-sh/bun@bun-v1.2.0", "--version"]);
        let config = GgConfig::default();
        let (cmds, _) = cli.parse_args(&config);
        assert_eq!(cmds[0].version.as_deref(), Some("1.2.0"));
        assert_eq!(cmds[0].distribution, None);
    }

    #[test]
    fn test_v_prefixed_version_is_normalised() {
        // A leading "v" must be stripped so the requirement matches the tag.
        let cli = parse_test_args(vec!["gh/oven-sh/bun@v1.2.0", "--version"]);
        let config = GgConfig::default();
        let (cmds, _) = cli.parse_args(&config);
        assert_eq!(cmds[0].version.as_deref(), Some("1.2.0"));
        assert_eq!(cmds[0].distribution, None);
    }

    #[test]
    fn test_partial_and_operator_versions_are_kept() {
        // Must not touch partial versions (ranges) or range operators
        let config = GgConfig::default();
        let (partial, _) = parse_test_args(vec!["node@1.2", "x"]).parse_args(&config);
        assert_eq!(partial[0].version.as_deref(), Some("1.2"));
        let (caret, _) = parse_test_args(vec!["node@^18", "x"]).parse_args(&config);
        assert_eq!(caret[0].version.as_deref(), Some("^18"));
    }

    #[test]
    fn test_distribution_only_still_parses() {
        let cli = parse_test_args(vec!["java@-temurin", "x"]);
        let config = GgConfig::default();
        let (cmds, _) = cli.parse_args(&config);
        assert_eq!(cmds[0].version, None);
        assert_eq!(cmds[0].distribution.as_deref(), Some("temurin"));
    }

    #[test]
    fn test_full_tag_with_digit_in_product_name() {
        // Digit in the product name must not be read as the version:
        // "log4j2-v2.20.0" pins 2.20.0, not "log4j2" or "2"
        let config = GgConfig::default();
        let (a, _) = parse_test_args(vec!["gh/apache/logging-log4j2@log4j2-v2.20.0", "x"])
            .parse_args(&config);
        assert_eq!(a[0].version.as_deref(), Some("2.20.0"));
        assert_eq!(a[0].distribution, None);
        let (b, _) = parse_test_args(vec!["gh/x/tool2@tool2-v1.2.3", "x"]).parse_args(&config);
        assert_eq!(b[0].version.as_deref(), Some("1.2.3"));
        assert_eq!(b[0].distribution, None);
    }

    #[test]
    fn test_operator_prefixes_are_normalised() {
        // The operator must survive stripping, or the req fails to parse and
        // falls back to latest
        let config = GgConfig::default();
        let (eq, _) = parse_test_args(vec!["gh/oven-sh/bun@=v1.2.0", "x"]).parse_args(&config);
        assert_eq!(eq[0].version.as_deref(), Some("=1.2.0"));
        let (caret, _) = parse_test_args(vec!["gh/oven-sh/bun@^v1.2.0", "x"]).parse_args(&config);
        assert_eq!(caret[0].version.as_deref(), Some("^1.2.0"));
        let (full, _) =
            parse_test_args(vec!["gh/oven-sh/bun@=bun-v1.2.0", "x"]).parse_args(&config);
        assert_eq!(full[0].version.as_deref(), Some("=1.2.0"));
    }

    #[test]
    fn test_distribution_kept_when_version_malformed() {
        // A malformed version must not make the parser drop the distribution
        // by mistaking the whole thing for a raw tag
        let cli = parse_test_args(vec!["java@1.2.3.4-temurin", "x"]);
        let config = GgConfig::default();
        let (cmds, _) = cli.parse_args(&config);
        assert_eq!(cmds[0].version.as_deref(), Some("1.2.3.4"));
        assert_eq!(cmds[0].distribution.as_deref(), Some("temurin"));
    }

    #[test]
    fn test_operator_version_keeps_distribution() {
        // A range operator on the version half must not send it down the
        // raw-tag path and drop the distribution.
        let config = GgConfig::default();
        let (caret, _) = parse_test_args(vec!["java@^17-temurin", "x"]).parse_args(&config);
        assert_eq!(caret[0].version.as_deref(), Some("^17"));
        assert_eq!(caret[0].distribution.as_deref(), Some("temurin"));
        let (eq, _) = parse_test_args(vec!["java@=1.2.0-temurin", "x"]).parse_args(&config);
        assert_eq!(eq[0].version.as_deref(), Some("=1.2.0"));
        assert_eq!(eq[0].distribution.as_deref(), Some("temurin"));
    }

    #[test]
    fn test_alias_expansion() {
        let cli = parse_test_args(vec!["build", "extra", "args"]);
        let mut config = GgConfig::default();
        config
            .aliases
            .insert("build".to_string(), "gradle clean build".to_string());

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "gradle");
        assert_eq!(app_args, vec!["clean", "build", "extra", "args"]);
    }

    #[test]
    fn test_alias_with_version() {
        let cli = parse_test_args(vec!["serve", "--port", "8080"]);
        let mut config = GgConfig::default();
        config
            .aliases
            .insert("serve".to_string(), "node@18 server.js".to_string());

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "18");
        assert_eq!(app_args, vec!["server.js", "--port", "8080"]);
    }

    #[test]
    fn test_dependency_version_resolution() {
        let cli = parse_test_args(vec!["node", "--version"]);
        let mut config = GgConfig::default();
        config
            .dependencies
            .insert("node".to_string(), "^18.0.0".to_string());

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "^18.0.0");
        assert_eq!(app_args, vec!["--version"]);
    }

    #[test]
    fn test_explicit_version_overrides_dependency() {
        let cli = parse_test_args(vec!["node@20", "--version"]);
        let mut config = GgConfig::default();
        config
            .dependencies
            .insert("node".to_string(), "^18.0.0".to_string());

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "20");
        assert_eq!(app_args, vec!["--version"]);
    }

    #[test]
    fn test_dependency_version_with_multiple_tools() {
        let cli = parse_test_args(vec!["node:java", "run"]);
        let mut config = GgConfig::default();
        config
            .dependencies
            .insert("node".to_string(), "^18.0.0".to_string());
        config
            .dependencies
            .insert("java".to_string(), "17".to_string());

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "node");
        assert_eq!(cmds[0].version.as_ref().unwrap(), "^18.0.0");
        assert_eq!(cmds[1].cmd, "java");
        assert_eq!(cmds[1].version.as_ref().unwrap(), "17");
        assert_eq!(app_args, vec!["run"]);
    }

    #[test]
    fn test_no_dependency_no_version() {
        let cli = parse_test_args(vec!["python", "--version"]);
        let config = GgConfig::default();

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "python");
        assert!(cmds[0].version.is_none());
        assert_eq!(app_args, vec!["--version"]);
    }

    #[test]
    fn test_run_with_gh_tool_is_rewritten() {
        // "run gh/owner/repo --arg" should become "gh/owner/repo --arg"
        let cli = parse_test_args(vec!["run", "gh/owner/repo", "--arg"]);
        let config = GgConfig::default();

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].cmd, "gh/owner/repo");
        assert_eq!(app_args, vec!["--arg"]);
    }

    #[test]
    fn test_run_with_known_tool_is_rewritten() {
        let cli = parse_test_args(vec!["run", "rat", "--help"]);
        let config = GgConfig::default();

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].cmd, "rat");
        assert_eq!(app_args, vec!["--help"]);
    }

    #[test]
    fn test_run_with_regular_command_unchanged() {
        let cli = parse_test_args(vec!["run", "somecommand"]);
        let config = GgConfig::default();

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].cmd, "run");
        assert_eq!(app_args, vec!["somecommand"]);
    }

    #[test]
    fn test_nested_alias_resolution() {
        let cli = parse_test_args(vec!["run-hello-chrome"]);
        let mut config = GgConfig::default();
        config.aliases.insert(
            "run-hello".to_string(),
            "flutter run -t lib/main.dart --flavor dev".to_string(),
        );
        config.aliases.insert(
            "run-hello-chrome".to_string(),
            "run-hello -d chrome".to_string(),
        );

        let (cmds, app_args) = cli.parse_args(&config);
        assert_eq!(cmds[0].cmd, "flutter");
        assert_eq!(
            app_args,
            vec![
                "run",
                "-t",
                "lib/main.dart",
                "--flavor",
                "dev",
                "-d",
                "chrome"
            ]
        );
    }
}
