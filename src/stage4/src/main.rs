use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::process::ExitCode;

use futures_util::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use log::{debug, info, LevelFilter};
use semver::VersionReq;

use crate::bloody_indiana_jones::download;
use crate::executor::{AppInput, Executor, ExecutorCmd, prep, try_run};
use crate::no_clap::NoClap;
use crate::target::Target;

mod target;
mod bloody_indiana_jones;
mod executor;
mod no_clap;
mod bloody_maven;
mod executors;

fn print_help(ver: &str) {
    println!(r"gg.cmd
https://github.com/eirikb/gg

Version: {ver}

Usage: ./gg.cmd [options] <executable name>@<version>:<dependent executable name>@<version> [program arguments]

Options:
    -v          Info output
    -vv         Debug output
    -vvv        Trace output
    -w          Even more output
    -V          Print version

Built in commands:
    update      Update gg.cmd
    help        Print help

Examples:
    ./gg.cmd node
    ./gg.cmd gradle@6:java@17 clean build
    ./gg.cmd node@10 -e 'console.log(1)'
    ./gg.cmd -vv npm@14 start
    ./gg.cmd java@-jdk+jre -version
    ./gg.cmd run soapui:java@17
    ./gg.cmd run env:java@14 java -version
    ./gg.cmd update

Supported systems:
    node (npm, npx will also work, version refers to node version)
    gradle
    java
    maven
    openapi
    rat (ra)
    run (any aritrary command)
");
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

    if let Some(cmd) = no_clap.cmds.first() {
        match cmd.cmd.as_str() {
            "update" => {
                println!("Updating gg.cmd...");
                let url = "https://github.com/eirikb/gg/releases/latest/download/gg.cmd";
                let pb = ProgressBar::new(0);
                download(url, "gg.cmd", &pb).await;
                return ExitCode::from(0);
            }
            "help" => {
                print_help(ver);
                return ExitCode::from(0);
            }
            _ => {}
        };
    }


    if no_clap.version {
        println!("{}", ver);
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &no_clap);

    let system = fs::read_to_string(format!("./.cache/gg/gg-{ver}/system")).unwrap_or(String::from("x86_64-linux")).trim().to_string();
    let target = Target::parse(&system);

    info!("System is {system}. {:?}", &target);

    let input = &AppInput { target, no_clap: no_clap.clone() };
    return if no_clap.cmds.first().is_some() {
        let mut executors = no_clap.cmds.iter().filter_map(|cmd| <dyn Executor>::new(ExecutorCmd {
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
            let mut path_vars: Vec<String> = vec!();

            let m = MultiProgress::new();

            let alles = executors.iter().enumerate().map(|(i, x)| {
                let pb = m.insert(i, ProgressBar::new(1));
                pb.set_style(ProgressStyle::with_template("{prefix:.bold} {spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                    .progress_chars("#>-"));
                (x, pb)
            }).map(|(x, pb)| async move {
                let app_path = prep(&**x, &input, &pb).await.expect("Prep failed");
                let env = x.get_env(&app_path);
                let bin_dirs = x.get_bin_dirs();
                (app_path, env, bin_dirs)
            });
            let res = join_all(alles).await;

            for (app_path, env, bin_dirs) in res.clone() {
                for bin_dir in &bin_dirs {
                    path_vars.push(app_path.install_dir.clone().join(bin_dir).to_str().unwrap_or("").to_string());
                }
                for (key, value) in env {
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }

            let (app_path, _, _) = &res[0];
            let executor = &executors[0];

            info!("Path vars: {}", &path_vars.join(", "));

            if try_run(input, &**executor, app_path.clone(), path_vars, env_vars).await.unwrap() {
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
        println!("Missing command. Try -h");
        print_help(ver);
        ExitCode::from(1)
    };
}
