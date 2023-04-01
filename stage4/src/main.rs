use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;

use futures_util::StreamExt;
use log::{debug, info, log};
use reqwest::Version;
use semver::VersionReq;

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::bloody_indiana_jones::download;
use crate::executor::{AppInput, Executor, ExecutorCmd, prep, try_run};
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
mod custom_command;

fn print_help(ver: &str) {
    println!(r"gg.cmd
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

    // let mut version_req_map = no_clap.version_req_map.clone();
    // let version_req_map2 = no_clap.version_req_map.clone();

    // for x in &version_req_map2 {
    //     let executor = cmd_to_executor(x.0.to_string());
    //     if let Some(executor) = executor {
    //         for x in executor.get_deps() {
    //             if !version_req_map.contains_key(x) {
    //                 version_req_map.insert(x.to_string(), Some(VersionReq::default()));
    //             }
    //         }
    //     }
    // }


    let input = &AppInput { target, no_clap: no_clap.clone() };
    return if let Some(cmd) = no_clap.cmds.first() {
        let mut executors = no_clap.cmds.iter().filter_map(|cmd| Executor::new(ExecutorCmd {
            cmd: cmd.cmd.to_string(),
            version: VersionReq::parse(cmd.version.clone().unwrap_or("".to_string()).as_str()).ok(),
            include_tags: cmd.include_tags.clone(),
            exclude_tags: cmd.exclude_tags.clone(),
        })).collect::<Vec<Box<dyn Executor>>>();

        let mut look_for_deps = true;
        while look_for_deps {
            look_for_deps = false;
            let mut to_add = Vec::new();
            for x in &executors {
                for dep_name in x.get_deps() {
                    if !executors.iter().any(|e| &e.get_name().to_string() == dep_name) {
                        if let Some(e) = Executor::new(ExecutorCmd {
                            cmd: dep_name.to_string(),
                            version: None,
                            include_tags: Default::default(),
                            exclude_tags: Default::default(),
                        }) {
                            look_for_deps = true;
                            to_add.push(e);
                        }
                    }
                }
            }
            for x in to_add {
                executors.push(x);
            }
        }

        let first = executors.first();
        if let Some(first) = executors.first() {
            let mut env_vars: HashMap<String, String> = HashMap::new();
            let mut path_vars: Vec<String> = vec!();

            for x in executors.iter().skip(1) {
                let app_path = prep(&**x, &input).await.expect("Prep failed");
                path_vars.push(app_path.parent_bin_path());
                for (key, value) in x.get_env(app_path) {
                    env_vars.insert(key, value);
                }
            }
            let app_path = prep(&**first, &input).await.expect("Prep failed");
            for (key, value) in first.get_env(app_path.clone()) {
                path_vars.push(app_path.clone().parent_bin_path());
                env_vars.insert(key, value);
            }

            dbg!(&env_vars);
            dbg!(&app_path);
            if app_path.bin.exists() {
                if try_run(input, app_path, path_vars, env_vars).await.unwrap() {
                    // Ok(())
                } else {
                    println!("Unable to execute");
                    ExitCode::from(1);
                };
            } else {
                println!("Binary not found!");
                ExitCode::from(1);
            }
        } else {
            println!("No executor found!");
            ExitCode::from(1);
        }


        // let bah: Vec<_> = executors.iter().collect();
        // let to_execute: Vec<_> = executors.iter_mut().collect();
        // while let Some(executor) = to_execute.pop() {
        //     for name in executor.get_deps() {
        //         if !bah.iter().any(|e| e.get_name() == name) {
        //             if let Some(dep_executor) = Executor::from_cmd(name) {
        //                 executors.push(dep_executor);
        //             }
        //         }
        //     }
        // }
        // // Run through deps once. Should be recursive
        // for executor in executors.into_iter() {
        //     for name in executor.get_deps() {
        //         if !executors.into_iter().any(|e| e.get_name() == name) {
        //             if let Some(dep_executor) = Executor::from_cmd(name) {
        //                 executors.push(dep_executor);
        //             }
        //         }
        //     }
        // }


        // let executor: Option<Box<dyn Executor>> = if no_clap.clone().custom_cmd {
        //     Some(Box::new(CustomCommand { cmd }))
        // } else {
        //     cmd_to_executor(cmd)
        // };

        // if executor.is_some() {
        //     // let c = no_clap.clone();
        //     // return ExitCode::from(if let Ok(_) = try_execute(&*executor.unwrap(), &AppInput { target, no_clap: c }, version_req_map).await {
        //     //     0
        //     // } else {
        //     //     1
        //     // });
        // } else {
        //     println!("Unable to find an executor for command. Try -h. Tip: If you just want to execute an arbitrary command try -c");
        // }
        ExitCode::from(1)
    } else {
        println!("Missing command. Try -h");
        print_help(ver);
        ExitCode::from(1)
    };
}
