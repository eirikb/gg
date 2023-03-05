use std::collections::{HashMap, HashSet};
use std::env;

use regex::Regex;
use semver::VersionReq;

/// Why not clap? Yes
#[derive(Debug, Clone)]
pub struct NoClap {
    pub gg_args: Vec<String>,
    pub app_args: Vec<String>,
    pub log_level: String,
    pub version_req_map: HashMap<String, Option<VersionReq>>,
    pub cmd: Option<String>,
    pub custom_cmd: bool,
    pub help: bool,
    pub version: bool,
    pub update: bool,
    pub include_tags: HashSet<String>,
    pub exclude_tags: HashSet<String>,
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
        let custom_cmd = gg_args.contains(&"-c".to_string()) || gg_args.contains(&"-e".to_string());
        let update = gg_args.contains(&"-u".to_string());

        let log_level = if let Some((_, log_level)) = log_level {
            log_level
        } else {
            "warn"
        }.to_string();

        let default_string = String::default();
        let cmds = cmds.unwrap_or(&default_string);

        let version_reqs_iter = cmds.split(":").filter(|s| !s.is_empty()).map(|cmd| {
            let parts: Vec<_> = Regex::new(r"@").unwrap().split(cmd).into_iter().collect();
            let cmd = parts[0].to_string();
            let version_req = VersionReq::parse(parts.get(1).unwrap_or(&"")).ok();
            (cmd, version_req)
        });

        let mut version_reqs: Vec<(String, Option<VersionReq>)> = version_reqs_iter.clone().collect();
        let cmd = if !version_reqs.is_empty() {
            let (cmd, _) = version_reqs.remove(0);
            Some(cmd)
        } else {
            None
        };

        let include_tags = HashSet::new();
        let exclude_tags = HashSet::new();

        let version_req_map: HashMap<String, Option<VersionReq>> = version_reqs_iter.into_iter().collect();

        Self { gg_args, app_args, log_level, cmd, version_req_map, help, version, custom_cmd, update, include_tags, exclude_tags }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node() {
        let no_clap = NoClap::parse(["node"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
    }

    #[test]
    fn node_with_args() {
        let no_clap = NoClap::parse(["node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
    }

    #[test]
    fn version_is_set() {
        let no_clap = NoClap::parse(["-V", "node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(false, no_clap.help);
    }

    #[test]
    fn version_is_set_but_not_help() {
        let no_clap = NoClap::parse(["-V", "node", "-h", "hello", "world"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
        assert_eq!(["-h", "hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(false, no_clap.help);
    }

    #[test]
    fn version_is_set_and_help() {
        let no_clap = NoClap::parse(["-V", "-h", "node", "hello", "world"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        assert_eq!(true, no_clap.version);
        assert_eq!(true, no_clap.help);
    }

    #[test]
    fn versioning_test() {
        let no_clap = NoClap::parse(["node@10:gradle@1.2.3", "hello", "world"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!("node", no_clap.cmd.unwrap());
        assert_eq!(["hello", "world"].map(String::from).to_vec(), no_clap.app_args);
        let mut map = HashMap::new();
        map.insert("node".to_string(), VersionReq::parse("10").ok());
        map.insert("gradle".to_string(), VersionReq::parse("1.2.3").ok());
        assert_eq!(map, no_clap.version_req_map);
    }

    #[test]
    fn print_help_no_cmd() {
        let no_clap = NoClap::parse(["-h"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_none());
        assert_eq!(true, no_clap.help);
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn print_version_no_cmd() {
        let no_clap = NoClap::parse(["-V"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_none());
        assert_eq!(false, no_clap.help);
        assert_eq!(true, no_clap.version);
    }

    #[test]
    fn custom_cmd1() {
        let no_clap = NoClap::parse(["-c"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_none());
        assert_eq!(true, no_clap.custom_cmd);
        assert_eq!(false, no_clap.version);
    }

    #[test]
    fn custom_cmd2() {
        let no_clap = NoClap::parse(["-c", "test"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_some());
        assert_eq!(true, no_clap.custom_cmd);
        assert_eq!(false, no_clap.version);
        assert_eq!("test", no_clap.cmd.unwrap());
    }

    #[test]
    fn update() {
        let no_clap = NoClap::parse(["-u"].map(String::from).to_vec());
        assert_eq!(true, no_clap.cmd.is_none());
        assert_eq!(true, no_clap.update);
        assert_eq!(false, no_clap.version);
    }

    // #[test]
    // fn include_tags() {
    //     let no_clap = NoClap::parse(["node@10:gradle@1.2.3+hello+world", "no", "problem"].map(String::from).to_vec());
    //     assert_eq!(true, no_clap.cmd.is_some());
    //     assert_eq!("node", no_clap.cmd.unwrap());
    //     assert_eq!(["no", "problem"].map(String::from).to_vec(), no_clap.app_args);
    //     let mut map = HashMap::new();
    //     map.insert("node".to_string(), VersionReq::parse("10").ok());
    //     map.insert("gradle".to_string(), VersionReq::parse("1.2.3").ok());
    //     assert_eq!(map, no_clap.version_req_map);
    //     let mut set = HashSet::new();
    //     set.insert("hello".to_string());
    //     set.insert("world".to_string());
    //     assert_eq!(set, no_clap.include_tags);
    // }
}
