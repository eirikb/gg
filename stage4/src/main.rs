use std::fs;
use std::process::{ExitCode};

use log::{debug, info};

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


fn print_help(ver: &str) {
    println!(r"gg.cmd - The Ultimate Executable Manager
https://github.com/eirikb/gg

Version: {ver}

Usage: ./gg.cmd [options] <executable name>@<version>:<dependent executable name>@<version> [program arguments]

Options:
    -u          Update gg.cmd
    -v          Verbose output
    -vv         Debug output
    -e          Execute first command blindly
    -c          Execute first command blindly
    -h          Print help
    -V          Print version

Examples:
    ./gg.cmd node
    ./gg.cmd -c soapui:java@17
    ./gg.cmd gradle@6:java@17 clean build
    ./gg.cmd node@10 -e 'console.log(1)'
    ./gg.cmd -vv npm@14 start

Supported systems:
    node (npm, npx will also work, version refers to node version)
    gradle
    java
");
}

#[tokio::main]
async fn main() -> ExitCode {
    let ver = option_env!("VER").unwrap_or("dev");

    let no_clap = NoClap::new();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or(&no_clap.log_level));

    if no_clap.help {
        print_help(ver);
        return ExitCode::from(0);
    }

    if no_clap.version {
        println!("{}", ver);
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &no_clap);

    let system = fs::read_to_string("./.cache/gg/system").unwrap_or(String::from("x86_64-linux")).trim().to_string();
    info!("System is {system}");

    let target = target::parse_target(&system);
    debug!(target: "main", "{:?}", &target);

    debug!(target: "main", "{:?}", &no_clap.version_req_map);

    let version_req_map = no_clap.version_req_map;

    return if let Some(cmd) = no_clap.cmd {
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
        ExitCode::from(1)
    } else {
        println!("Missing command");
        print_help(ver);
        ExitCode::from(1)
    };
}
