use std::{env, fs};
use std::collections::HashMap;
use semver::VersionReq;
use regex::Regex;

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::executor::{AppInput, Executor, try_execute};
use crate::gradle::Gradle;
use crate::java::Java;
use crate::node::Node;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;
mod java;
mod executor;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("x86_64-linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = target::parse_target(&system);
    dbg!(&target);

    match args.get(1) {
        Some(cmds) => {
            let version_reqs_iter = cmds.split(":").map(|cmd| {
                let parts: Vec<_> = Regex::new(r"[@]").unwrap().split(cmd).into_iter().collect();
                let cmd = parts[0].to_string();
                let version_req = VersionReq::parse(parts.get(1).unwrap_or(&"*")).unwrap_or(VersionReq::default());
                (cmd, version_req)
            });
            let mut version_reqs: Vec<(String, VersionReq)> = version_reqs_iter.clone().collect();
            dbg!(version_reqs.clone());
            let (cmd, _) = version_reqs.remove(0);
            let version_req_map: HashMap<String, VersionReq> = version_reqs_iter.into_iter().collect();

            let executor: Option<Box<dyn Executor>> = match cmd.as_str() {
                "node" | "npm" | "npx" => Some(Box::new(Node { cmd, version_req_map })),
                "gradle" => Some(Box::new(Gradle { version_req_map })),
                "java" => Some(Box::new(Java { version_req_map })),
                _ => None
            };

            if executor.is_some() {
                try_execute(&*executor.unwrap(), &AppInput { target }).await.unwrap();
            }
        }
        None => {
            println!("No command")
        }
    };
}
