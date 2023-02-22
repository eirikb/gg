use std::fs;
use std::process::{ExitCode};

use log::{debug, info};
use semver::VersionReq;

use bloody_indiana_jones::download_unpack_and_all_that_stuff;
use crate::bloody_indiana_jones::download;

use crate::executor::{AppInput, Executor, try_execute};
use crate::gradle::Gradle;
use crate::java::Java;
use crate::no_clap::NoClap;
use crate::node::Node;
use crate::custom_command::CustomCommand;
use crate::cmd_to_executor::cmd_to_executor;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;
mod java;
mod executor;
mod no_clap;
mod custom_command;
mod cmd_to_executor;


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
    let ver = option_env!("VERSION").unwrap_or("dev");

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

    if no_clap.update {
        println!("Updating gg.cmd...");
        let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
        download(url, "gg.cmd").await;
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &no_clap);

    let system = fs::read_to_string(format!("./.cache/gg-{ver}/system")).unwrap_or(String::from("x86_64-linux")).trim().to_string();
    info!("System is {system}");

    let target = target::parse_target(&system);
    debug!(target: "main", "{:?}", &target);

    debug!(target: "main", "{:?}", &no_clap.version_req_map);

    let mut version_req_map = no_clap.version_req_map.clone();
    let version_req_map2 = no_clap.version_req_map.clone();

    for x in &version_req_map2 {
        let executor = cmd_to_executor(x.0.to_string(), version_req_map.clone());
        if let Some(executor) = executor {
            for x in executor.get_deps() {
                if !version_req_map.contains_key(x) {
                    version_req_map.insert(x.to_string(), Some(VersionReq::default()));
                }
            }
        }
    }


    return if let Some(cmd) = no_clap.clone().cmd {
        let executor: Option<Box<dyn Executor>> = if no_clap.clone().custom_cmd {
            Some(Box::new(CustomCommand { cmd }))
        } else {
            cmd_to_executor(cmd, version_req_map.clone())
        };

        if executor.is_some() {
            let c = no_clap.clone();
            return ExitCode::from(if let Ok(_) = try_execute(&*executor.unwrap(), &AppInput { target, no_clap: c }, version_req_map).await {
                0
            } else {
                1
            });
        } else {
            println!("Unable to find an executor for command. Try -h. Tip: If you just want to execute an arbitrary command try -c");
        }
        ExitCode::from(1)
    } else {
        println!("Missing command. Try -h");
        print_help(ver);
        ExitCode::from(1)
    };
}
