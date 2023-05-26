use bloody_indiana_jones::download_unpack_and_all_that_stuff;
use futures_util::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use log::{debug, info};
use semver::VersionReq;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::process::ExitCode;

use crate::bloody_indiana_jones::download;
use crate::executor::{AppInput, Executor, ExecutorCmd, prep, try_run};
use crate::gradle::Gradle;
use crate::java::Java;
use crate::no_clap::NoClap;
use crate::node::Node;
use crate::target::Target;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;
mod maven;
mod openapigenerator;
mod java;
mod executor;
mod no_clap;
mod custom_command;
mod bloody_maven;

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
    ./gg.cmd java@-jdk+jre -version

Supported systems:
    node (npm, npx will also work, version refers to node version)
    gradle
    java
    maven
    openapi
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
        let pb = ProgressBar::new(0);
        download(url, "gg.cmd", &pb).await;
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &no_clap);

    let system = fs::read_to_string(format!("./.cache/gg-{ver}/system")).unwrap_or(String::from("x86_64-linux")).trim().to_string();
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
                let p = app_path.clone();
                (app_path, x.get_env(p))
            });
            let res = join_all(alles).await;

            for (app_path, env) in res.clone() {
                for (key, value) in env {
                    path_vars.push(app_path.clone().parent_bin_path());
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }

            let (app_path, _) = &res[0];
            let executor = &executors[0];

            if app_path.bin.exists() {
                if try_run(input, &**executor, app_path.clone(), path_vars, env_vars).await.unwrap() {
                    ExitCode::from(0)
                } else {
                    println!("Unable to execute");
                    ExitCode::from(1)
                }
            } else {
                println!("Binary not found!");
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
