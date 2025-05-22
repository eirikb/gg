use std::collections::HashSet;
use std::env;

use regex::{Match, Regex};

#[derive(Debug, Clone)]
pub struct NoClapCmd {
    pub cmd: String,
    pub version: Option<String>,
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
            } else if (arg == "--os" || arg == "--arch") && i + 1 < args.len() {
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

        let version = gg_args.contains(&"-V".to_string());
        let log_external = gg_args.contains(&"-w".to_string());

        let override_os = Self::extract_flag_value(&gg_args, "--os");
        let override_arch = Self::extract_flag_value(&gg_args, "--arch");

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

                if parts.len() == 2 {
                    cmd = parts[0].to_string();

                    let r = Regex::new(r"[+-]").unwrap();
                    let alles = parts[1].to_string();
                    let matches = r.find_iter(&alles).collect::<Vec<Match>>();
                    if matches.is_empty() {
                        version = Some(alles.clone());
                    }
                    matches.iter().enumerate().for_each(|(index, m)| {
                        if index == 0 && m.start() != 0 {
                            version = Some(alles[0..m.start()].to_string());
                        }
                        let until = if index < matches.len() - 1 {
                            matches[index + 1].start()
                        } else {
                            alles.len()
                        };
                        let command = alles[m.start()..m.start() + 1].to_string();
                        let text = alles[m.start() + 1..until].to_string();
                        if command == "+" {
                            include_tags.insert(text);
                        } else if command == "-" {
                            exclude_tags.insert(text);
                        }
                    });
                }

                NoClapCmd {
                    cmd,
                    version,
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
}
