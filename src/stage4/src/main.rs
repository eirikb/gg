use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;

use futures_util::future::join_all;
use indicatif::MultiProgress;
use log::{debug, info, LevelFilter};

use crate::barus::create_barus;
use crate::bloody_indiana_jones::BloodyIndianaJones;
use crate::executor::{prep, try_run, AppInput, Executor, ExecutorCmd, GgVersionReq};
use crate::no_clap::NoClap;
use crate::target::Target;

mod barus;
mod bloody_indiana_jones;
mod bloody_maven;
mod checker;
mod executor;
mod executors;
mod no_clap;
mod target;

fn print_help(ver: &str) {
    println!(
        r"
https://github.com/eirikb/gg

Version: {ver}

Usage: ./gg.cmd [options] <executable name>@<version>:<dependent executable name>@<version> [program arguments]

Options:
    -v              Info output
    -vv             Debug output
    -vvv            Trace output
    -w              Even more output
    -V              Print version

Built in commands:
    update          Update gg.cmd
    help            Print help
    check           Check for updates
    check-update    Check for updates and update if available
    clean-cache     Clean cache

Version syntax:
    @X              Any X.y.z version (e.g. node@14 for any Node.js 14.x.y)
    @X.Y            Any X.Y.z patch version (e.g. node@14.17 for any Node.js 14.17.z)
    @X.Y.Z          Exactly X.Y.Z version (e.g. node@14.17.0 for exactly Node.js 14.17.0)
    @^X.Y.Z         X.Y.Z or any compatible newer version (caret prefix)
    @~X.Y.Z         X.Y.Z or any newer patch version (tilde prefix)
    @=X.Y.Z         Exactly X.Y.Z version (equals prefix, same as X.Y.Z without prefix)

Examples:
    ./gg.cmd node
    ./gg.cmd gradle@6:java@17 clean build
    ./gg.cmd node@10 -e 'console.log(1)'
    ./gg.cmd node@14.17.0 -v
    ./gg.cmd -vv -w npm@14 start
    ./gg.cmd java@-jdk+jre -version
    ./gg.cmd run:java@17 soapui
    ./gg.cmd run:java@14 env
    ./gg.cmd update

Supported systems:
    node (npm, npx will also work, version refers to node version)
    gradle
    java
    maven
    openapi
    rat (ra)
    deno
    run (any arbitrary command)
    go
    caddy
"
    );
}

#[tokio::main]
async fn main() -> ExitCode {
    let ver = option_env!("VERSION").unwrap_or("dev");

    let no_clap = NoClap::new();
    let log_level = match no_clap.log_level.as_str() {
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        _ => LevelFilter::Warn,
    };
    if no_clap.log_external {
        env_logger::builder().filter_level(log_level).init();
    } else {
        env_logger::builder()
            .filter_module("gg", log_level)
            .filter_module("gg:executors", log_level)
            .filter_module("stage4", log_level)
            .filter_module("stage4:executors", log_level)
            .init();
    }

    let system = fs::read_to_string(format!("./.cache/gg/gg-{ver}/system"))
        .unwrap_or(String::from("x86_64-linux"))
        .trim()
        .to_string();
    let target = Target::parse(&system);

    let input = &AppInput {
        target,
        no_clap: no_clap.clone(),
    };

    if no_clap.version {
        println!("{}", ver);
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &no_clap);

    if let Some(cmd) = no_clap.cmds.first() {
        match cmd.cmd.as_str() {
            "update" => {
                println!("Updating gg.cmd...");
                let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
                let pb = create_barus();
                let bloody_indiana_jones = BloodyIndianaJones::new_with_file_name(
                    url.to_string(),
                    "gg.cmd".to_string(),
                    pb.clone(),
                );
                bloody_indiana_jones.download().await;
                return ExitCode::from(0);
            }
            "help" => {
                print_help(ver);
                return ExitCode::from(0);
            }
            "check" => {
                checker::check(input, false).await;
                return ExitCode::from(0);
            }
            "check-update" => {
                checker::check(input, true).await;
                return ExitCode::from(0);
            }
            "clean-cache" => {
                println!("Cleaning cache");
                let _ = fs::remove_dir_all(".cache/gg");
                return ExitCode::from(0);
            }
            _ => {}
        };
    }

    info!("System is {system}. {:?}", &target);

    return if no_clap.cmds.first().is_some() {
        let mut executors = no_clap
            .cmds
            .iter()
            .filter_map(|cmd| {
                <dyn Executor>::new(ExecutorCmd {
                    cmd: cmd.cmd.to_string(),
                    version: GgVersionReq::new(
                        cmd.version.clone().unwrap_or("".to_string()).as_str(),
                    ),
                    include_tags: cmd.include_tags.clone(),
                    exclude_tags: cmd.exclude_tags.clone(),
                })
            })
            .collect::<Vec<Box<dyn Executor>>>();

        let mut look_for_deps = true;
        while look_for_deps {
            look_for_deps = false;
            let mut to_add = Vec::new();
            for x in &executors {
                for dep_name in x.get_deps() {
                    if !executors
                        .iter()
                        .any(|e| &e.get_name().to_string() == dep_name)
                    {
                        if let Some(e) = <dyn Executor>::new(ExecutorCmd {
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

        return if executors.first().is_some() {
            let mut env_vars: HashMap<String, String> = HashMap::new();
            let mut path_vars: Vec<String> = vec![];

            let m = MultiProgress::new();

            let alles = executors
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let pb = create_barus();
                    let pb = m.insert(i, pb);
                    (x, pb)
                })
                .map(|(x, pb)| async move {
                    let app_path = prep(&**x, &input, &pb).await?;
                    let env = x.get_env(&app_path);
                    let bin_dirs = x.get_bin_dirs();
                    Ok::<_, String>((app_path, env, bin_dirs))
                });
            let res = join_all(alles).await;

            res.iter().filter(|x| x.is_err()).for_each(|x| {
                println!("Prep failed: {}", x.clone().err().unwrap());
            });
            if res.iter().any(|x| x.is_err()) {
                return ExitCode::from(1);
            }

            let res = res.into_iter().filter_map(|x| x.ok()).collect::<Vec<_>>();

            for (app_path, env, bin_dirs) in res.clone() {
                for bin_dir in &bin_dirs {
                    path_vars.push(
                        app_path
                            .install_dir
                            .clone()
                            .join(bin_dir)
                            .to_str()
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                for (key, value) in env {
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }

            let (app_path, _, _) = &res[0];
            let executor = &executors[0];

            info!("Path vars: {}", &path_vars.join(", "));

            if try_run(input, &**executor, app_path.clone(), path_vars, env_vars)
                .await
                .unwrap()
            {
                ExitCode::from(0)
            } else {
                println!("Unable to execute");
                ExitCode::from(1)
            }
        } else {
            println!("No executor found!");
            ExitCode::from(1)
        };
    } else {
        println!("Missing command. Try help");
        print_help(ver);
        ExitCode::from(1)
    };
}
