use std::collections::HashSet;
use std::env;

use regex::{Match, Regex};

#[derive(Debug, Clone)]
pub struct NoClapCmd {
    pub cmd: String,
    pub version: Option<String>,
    pub distribution: Option<String>,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
}

/// Why not clap? Yes
#[derive(Debug, Clone)]
pub struct NoClap {
    pub app_args: Vec<String>,
    pub log_level: String,
    pub log_external: bool,
    pub cmds: Vec<NoClapCmd>,
    pub version: bool,
    pub override_os: Option<String>,
    pub override_arch: Option<String>,
    pub cache_dir: Option<String>,
}

impl NoClap {
    pub fn new() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        NoClap::parse(args)
    }

    // Ok. So. Now I kind of regret no_clap over clap
    // but I had my reasons. Honestly
    pub fn parse(args: Vec<String>) -> Self {
        let mut start_at = args.len();
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            if !arg.starts_with("-") {
                start_at = i;
                break;
            } else if (arg == "--os" || arg == "--arch" || arg == "--cache-dir") && i + 1 < args.len() {
                i += 2;
            } else {
                i += 1;
            }
        }
        let cmds = args.get(start_at);
        let gg_args: Vec<String> = args.clone().into_iter().take(start_at).collect();
        let app_args: Vec<String> = args.clone().into_iter().skip(start_at + 1).collect();
        let log_level = vec![("-vvv", "trace"), ("-vv", "debug"), ("-v", "info")]
            .into_iter()
            .find(|(input, _)| gg_args.contains(&input.to_string()));

        let version =
            gg_args.contains(&"-V".to_string()) || gg_args.contains(&"--version".to_string());
        let log_external = gg_args.contains(&"-w".to_string());

        let override_os = Self::extract_flag_value(&gg_args, "--os");
        let override_arch = Self::extract_flag_value(&gg_args, "--arch");
        let cache_dir = Self::extract_flag_value(&gg_args, "--cache-dir");

        let log_level = if let Some((_, log_level)) = log_level {
            log_level
        } else {
            "warn"
        }
        .to_string();

        let default_string = String::default();
        let cmds = cmds.unwrap_or(&default_string);

        let cmds = cmds
            .split(":")
            .filter(|s| !s.is_empty())
            .map(|cmd| {
                let mut cmd = cmd.to_string();
                let parts: Vec<String> = cmd.split("@").map(String::from).collect();
                let mut include_tags = HashSet::new();
                let mut exclude_tags = HashSet::new();
                let mut version = None;
                let mut distribution = None;

                if parts.len() == 2 {
                    cmd = parts[0].to_string();

                    // Ok. Bah. A bit of hack here because of tags
                    // Plan: Look for + or - that are NOT part of a version-distribution
                    // We'll find the first + or - that comes after a space or after a distribution name
                    // Meaning @ver-var should mean ver-var is a version-distribution. I hope

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

                NoClapCmd {
                    cmd,
                    version,
                    distribution,
                    include_tags,
                    exclude_tags,
                }
            })
            .collect();

        Self {
            app_args,
            log_level,
            log_external,
            cmds,
            version,
            override_os,
            override_arch,
            cache_dir,
        }
    }

    fn extract_flag_value(args: &Vec<String>, flag: &str) -> Option<String> {
        for (i, arg) in args.iter().enumerate() {
            if arg == flag && i + 1 < args.len() {
                return Some(args[i + 1].clone());
            } else if arg.starts_with(&format!("{}=", flag)) {
                return Some(arg[flag.len() + 1..].to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_with_args() {
        let no_clap = NoClap::parse(["node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(
            ["hello", "world"].map(String::from).to_vec(),
            no_clap.app_args
        );
    }

    #[test]
    fn version_is_set() {
        let no_clap = NoClap::parse(["-V", "node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(
            ["hello", "world"].map(String::from).to_vec(),
            no_clap.app_args
        );
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn version_is_set_but_not_help() {
        let no_clap = NoClap::parse(
            ["-V", "node", "-h", "hello", "world"]
                .map(String::from)
                .to_vec(),
        );
        assert_eq!(
            ["-h", "hello", "world"].map(String::from).to_vec(),
            no_clap.app_args
        );
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn version_is_set_and_help() {
        let no_clap = NoClap::parse(
            ["-V", "-h", "node", "hello", "world"]
                .map(String::from)
                .to_vec(),
        );
        assert_eq!(
            ["hello", "world"].map(String::from).to_vec(),
            no_clap.app_args
        );
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn check_update() {
        let no_clap = NoClap::parse(["check-update"].map(String::from).to_vec());
        assert_eq!("check-update", no_clap.cmds.first().unwrap().cmd);
    }

    #[test]
    fn versioning_test() {
        let no_clap = NoClap::parse(
            ["node@10:gradle@1.2.3", "hello", "world"]
                .map(String::from)
                .to_vec(),
        );
        assert_eq!(
            ["hello", "world"].map(String::from).to_vec(),
            no_clap.app_args
        );

        assert_eq!("node", no_clap.cmds[0].cmd);
        assert_eq!("10", no_clap.cmds[0].version.as_ref().unwrap());

        assert_eq!("gradle", no_clap.cmds[1].cmd);
        assert_eq!("1.2.3", no_clap.cmds[1].version.as_ref().unwrap());
    }

    #[test]
    fn test_os_arch_overrides() {
        let no_clap = NoClap::parse(
            [
                "--os",
                "windows",
                "--arch",
                "arm64",
                "-v",
                "deno",
                "--version",
            ]
            .map(String::from)
            .to_vec(),
        );
        assert_eq!(Some("windows".to_string()), no_clap.override_os);
        assert_eq!(Some("arm64".to_string()), no_clap.override_arch);
        assert_eq!("deno", no_clap.cmds[0].cmd);
        assert_eq!(["--version"].map(String::from).to_vec(), no_clap.app_args);
    }

    #[test]
    fn print_help_no_cmd() {
        let no_clap = NoClap::parse(["-h"].map(String::from).to_vec());
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn print_version_no_cmd() {
        let no_clap = NoClap::parse(["-V"].map(String::from).to_vec());
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn java() {
        let no_clap = NoClap::parse(["-v", "java@11", "-version"].map(String::from).to_vec());
        assert_eq!("11", no_clap.cmds[0].version.as_ref().unwrap());
    }

    #[test]
    fn include_tags() {
        let no_clap = NoClap::parse(
            [
                "node@10:gradle@1.2.3+hello+world:java@+azul",
                "no",
                "problem",
            ]
            .map(String::from)
            .to_vec(),
        );

        assert_eq!(
            ["no", "problem"].map(String::from).to_vec(),
            no_clap.app_args
        );

        assert_eq!("node", no_clap.cmds[0].cmd);
        assert_eq!("10", no_clap.cmds[0].version.as_ref().unwrap());

        assert_eq!("gradle", no_clap.cmds[1].cmd);
        assert_eq!("1.2.3", no_clap.cmds[1].version.as_ref().unwrap());

        assert_eq!("java", no_clap.cmds[2].cmd);
        assert_eq!(None, no_clap.cmds[2].version);
    }

    #[test]
    fn distribution_parsing() {
        let no_clap = NoClap::parse(["java@17-temurin", "hello"].map(String::from).to_vec());

        assert_eq!("java", no_clap.cmds[0].cmd);
        assert_eq!("17", no_clap.cmds[0].version.as_ref().unwrap());
        assert_eq!("temurin", no_clap.cmds[0].distribution.as_ref().unwrap());
    }

    #[test]
    fn distribution_with_tags() {
        let no_clap = NoClap::parse(
            ["java@17-temurin+lts-ea", "hello"]
                .map(String::from)
                .to_vec(),
        );

        assert_eq!("java", no_clap.cmds[0].cmd);
        assert_eq!("17", no_clap.cmds[0].version.as_ref().unwrap());
        assert_eq!("temurin", no_clap.cmds[0].distribution.as_ref().unwrap());
        assert!(no_clap.cmds[0].include_tags.contains("lts"));
        assert!(no_clap.cmds[0].exclude_tags.contains("ea"));
    }

    #[test]
    fn distribution_short_form() {
        let no_clap = NoClap::parse(["java@20.0.2-tem", "hello"].map(String::from).to_vec());

        assert_eq!("java", no_clap.cmds[0].cmd);
        assert_eq!("20.0.2", no_clap.cmds[0].version.as_ref().unwrap());
        assert_eq!("tem", no_clap.cmds[0].distribution.as_ref().unwrap());
    }
}
