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
    pub gg_args: Vec<String>,
    pub app_args: Vec<String>,
    pub log_level: String,
    pub cmds: Vec<NoClapCmd>,
    pub help: bool,
    pub version: bool,
    pub update: bool,
}

impl NoClap {
    pub fn new() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        return NoClap::parse(args);
    }

    pub fn parse(args: Vec<String>) -> Self {
        let start_at = args.iter().position(|item| !item.starts_with("-")).unwrap_or(args.len());
        let cmds = args.get(start_at);
        let gg_args: Vec<String> = args.clone().into_iter().take(start_at).collect();
        let app_args: Vec<String> = args.clone().into_iter().skip(start_at + 1).collect();
        let log_level = vec![("-vv", "debug"), ("-v", "info")].into_iter().find(|(input, _)| gg_args.contains(&input.to_string()));

        let help = gg_args.contains(&"-h".to_string());
        let version = gg_args.contains(&"-V".to_string());
        let update = gg_args.contains(&"-u".to_string());

        let log_level = if let Some((_, log_level)) = log_level {
            log_level
        } else {
            "warn"
        }.to_string();

        let default_string = String::default();
        let cmds = cmds.unwrap_or(&default_string);

        let cmds = cmds.split(":").filter(|s| !s.is_empty()).map(|cmd| {
            let mut cmd = cmd.to_string();
            let r = Regex::new(r"[@+-]").unwrap();

            let alles = cmd.clone();
            let matches = r.find_iter(&alles).collect::<Vec<Match>>();

            let mut include_tags = HashSet::new();
            let mut exclude_tags = HashSet::new();
            let mut version = None;

            matches.iter().enumerate().for_each(|(index, m)| {
                if index == 0 {
                    cmd = cmd.as_str()[0..m.end() - 1].to_string();
                }

                let until = if index < matches.len() - 1 { matches[index + 1].start() } else { alles.len() };
                let command = alles[m.start()..m.start() + 1].to_string();
                let text = alles[m.start() + 1..until].to_string();
                if command == "+" {
                    include_tags.insert(text);
                } else if command == "-" {
                    exclude_tags.insert(text);
                } else if command == "@" {
                    version = Some(text);
                }
            });

            NoClapCmd {
                cmd,
                version,
                include_tags,
                exclude_tags,
            }
        }).collect();

        Self { gg_args, app_args, log_level, cmds, help, version, update }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_with_args() {
        let no_clap = NoClap::parse(["node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
    }

    #[test]
    fn version_is_set() {
        let no_clap = NoClap::parse(["-V", "node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(false, no_clap.help);
    }

    #[test]
    fn version_is_set_but_not_help() {
        let no_clap = NoClap::parse(["-V", "node", "-h", "hello", "world"].map(String::from).to_vec());
        assert_eq!(["-h", "hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(false, no_clap.help);
    }

    #[test]
    fn version_is_set_and_help() {
        let no_clap = NoClap::parse(["-V", "-h", "node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(true, no_clap.help);
    }

    #[test]
    fn versioning_test() {
        let no_clap = NoClap::parse(["node@10:gradle@1.2.3", "hello", "world"].map(String::from).to_vec());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);

        assert_eq!("node", no_clap.cmds[0].cmd);
        assert_eq!("10", no_clap.cmds[0].version.as_ref().unwrap());

        assert_eq!("gradle", no_clap.cmds[1].cmd);
        assert_eq!("1.2.3", no_clap.cmds[1].version.as_ref().unwrap());
    }

    #[test]
    fn print_help_no_cmd() {
        let no_clap = NoClap::parse(["-h"].map(String::from).to_vec());
        assert_eq!(true, no_clap.help);
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn print_version_no_cmd() {
        let no_clap = NoClap::parse(["-V"].map(String::from).to_vec());
        assert_eq!(false, no_clap.help);
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn custom_cmd1() {
        let no_clap = NoClap::parse(["-c"].map(String::from).to_vec());
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn custom_cmd2() {
        let no_clap = NoClap::parse(["-c", "test"].map(String::from).to_vec());
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn update() {
        let no_clap = NoClap::parse(["-u"].map(String::from).to_vec());
        assert_eq!(true, no_clap.update);
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn java() {
        let no_clap = NoClap::parse(["-v", "java@11", "-version"].map(String::from).to_vec());
        assert_eq!("11", no_clap.cmds[0].version.as_ref().unwrap());
    }

    #[test]
    fn include_tags() {
        let no_clap = NoClap::parse(["node@10:gradle@1.2.3+hello+world:java+azul", "no", "problem"].map(String::from).to_vec());

        assert_eq!(["no", "problem"].map(String::from).to_vec(), no_clap.app_args);

        assert_eq!("node", no_clap.cmds[0].cmd);
        assert_eq!("10", no_clap.cmds[0].version.as_ref().unwrap());

        assert_eq!("gradle", no_clap.cmds[1].cmd);
        assert_eq!("1.2.3", no_clap.cmds[1].version.as_ref().unwrap());

        assert_eq!("java", no_clap.cmds[2].cmd);
        assert_eq!(None, no_clap.cmds[2].version);
    }
}
