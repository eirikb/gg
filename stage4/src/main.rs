use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;

use log::{debug, info};
use regex::Regex;
use semver::VersionReq;

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::executor::{AppInput, Executor, try_execute};
use crate::gradle::Gradle;
use crate::java::Java;
use crate::no_clap::NoClap;
use crate::node::Node;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;
mod java;
mod executor;
mod no_clap;


#[tokio::main]
async fn main() -> ExitCode {
    let ver = option_env!("VER").unwrap_or("dev");
    println!("Version is {ver}");

    if let Some(no_clap) = NoClap::new() {
        let log_level = vec![("-vv", "debug"), ("-v", "info")].into_iter().find(|(input, _)| no_clap.gg_args.contains(&input.to_string()));

        let log_level = if let Some((_, log_level)) = log_level {
            log_level
        } else {
            "warn"
        };

        env_logger::init_from_env(env_logger::Env::default().default_filter_or(log_level));

        debug!(target: "main", "{:?}", &no_clap);

        let system = fs::read_to_string("./.cache/gg/system").unwrap_or(String::from("x86_64-linux")).trim().to_string();
        info!("System is {system}");

        let target = target::parse_target(&system);
        debug!(target: "main", "{:?}", &target);

        let version_reqs_iter = no_clap.cmds.split(":").map(|cmd| {
            let parts: Vec<_> = Regex::new(r"@").unwrap().split(cmd).into_iter().collect();
            let cmd = parts[0].to_string();
            let version_req = VersionReq::parse(parts.get(1).unwrap_or(&"")).ok();
            (cmd, version_req)
        });

        let mut version_reqs: Vec<(String, Option<VersionReq>)> = version_reqs_iter.clone().collect();
        let (cmd, _) = version_reqs.remove(0);
        let version_req_map: HashMap<String, Option<VersionReq>> = version_reqs_iter.into_iter().collect();
        debug!(target: "main", "{:?}", &version_req_map);

        let executor: Option<Box<dyn Executor>> = match cmd.as_str() {
            "node" | "npm" | "npx" => Some(Box::new(Node { cmd, version_req_map })),
            "gradle" => Some(Box::new(Gradle { version_req_map })),
            "java" => Some(Box::new(Java { version_req_map })),
            _ => None
        };

        if executor.is_some() {
            return ExitCode::from(if let Ok(_) = try_execute(&*executor.unwrap(), &AppInput { target }).await {
                0
            } else {
                1
            });
        } else {
            info!("Unable to find an executor");
        }
    } else {
        println!("No command");
        println!("Here be help in the future. I promise");
    }
    return ExitCode::from(1);
}
