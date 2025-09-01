use crate::config::GgConfig;
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
                        }],
                        vec![],
                    )
                }
            }
        } else {
            if let Some(first_arg) = self.args.first() {
                if let Some(alias_args) = config.resolve_alias(first_arg) {
                    let mut expanded_args = alias_args;
                    expanded_args.extend_from_slice(&self.args[1..]);

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
                    (cmds, app_args)
                }
            } else {
                (vec![], vec![])
            }
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
                    let version_str = version_dist_part[0..dash_pos].to_string();
                    version = if version_str.is_empty() {
                        None
                    } else {
                        Some(version_str)
                    };
                    distribution = Some(version_dist_part[dash_pos + 1..].to_string());
                } else {
                    version = if version_dist_part.is_empty() {
                        None
                    } else {
                        Some(version_dist_part)
                    };
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
}
