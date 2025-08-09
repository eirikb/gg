use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::ExitCode;

use futures_util::future::join_all;
use indicatif::MultiProgress;
use log::{debug, info, LevelFilter};

use crate::barus::create_barus;
use crate::cli::Cli;
use crate::executor::{prep, try_run, AppInput, Executor, ExecutorCmd, GgVersionReq};
use crate::target::Target;
use clap::Parser;

mod barus;
mod bloody_indiana_jones;
mod bloody_maven;
mod checker;
mod cleaner;
mod cli;
mod executor;
mod executors;
mod target;
mod tools;
mod updater;

use crate::tools::{get_all_tools, get_tool_info, ToolCategory};

fn print_help(ver: &str) {
    println!(
        r"
https://github.com/eirikb/gg

Version: {ver}

Usage: ./gg.cmd [options] <executable name>@<version>:<dependent executable name>@<version> [program arguments]

Options:
    -l              Use local cache (.cache/gg) instead of global cache
    -v              Info output
    -vv             Debug output
    -vvv            Trace output
    -w              Even more output
    -h, --help      Print help
    -V, --version   Print version
    --os <OS>       Override target OS (windows, linux, mac)
    --arch <ARCH>   Override target architecture (x86_64, arm64, armv7)

Built in commands:
    update          Check for updates for all tools (including gg)
    update -u       Update all tools that have updates available
    update <tool>   Check for updates for specific tool (e.g., update flutter, update gg)
    update <tool> -u Update specific tool (e.g., update flutter -u, update gg -u)
    update <tool> -u -f Force update specific tool even if up to date (e.g., update gg -u -f)
    help            Print help
    tools           List all available tools
    clean-cache     Clean cache (prompts for confirmation)

Update options:
    -u              Actually perform the update (vs just checking)
    -f              Force re-download even if already up to date (use with -u)
    --major         Include major version updates (default: skip major versions)

Version syntax:
    @X              Any X.y.z version (e.g. node@14 for any Node.js 14.x.y)
    @X.Y            Any X.Y.z patch version (e.g. node@14.17 for any Node.js 14.17.z)
    @X.Y.Z          Exactly X.Y.Z version (e.g. node@14.17.0 for exactly Node.js 14.17.0)
    @^X.Y.Z         X.Y.Z or any compatible newer version (caret prefix)
    @~X.Y.Z         X.Y.Z or any newer patch version (tilde prefix)
    @=X.Y.Z         Exactly X.Y.Z version (equals prefix, same as X.Y.Z without prefix)

Examples:
    ./gg.cmd node
    ./gg.cmd -l node                                      (use local cache)
    ./gg.cmd gradle@6:java@17 clean build
    ./gg.cmd -l gradle@6:java@17 clean build             (use local cache)
    ./gg.cmd node@10 -e 'console.log(1)'
    ./gg.cmd node@14.17.0 -v
    ./gg.cmd -vv -w npm@14 start
    ./gg.cmd java@-jdk+jre -version
    ./gg.cmd jbang hello.java
    ./gg.cmd bld version
    ./gg.cmd maven compile
    ./gg.cmd run:java@17 soapui
    ./gg.cmd run:java@14 env
    ./gg.cmd update
    ./gg.cmd gh/cli/cli --version
    ./gg.cmd --os windows --arch x86_64 deno --version    (test Windows Deno on Linux)
    ./gg.cmd --os mac deno --help                         (test macOS Deno from anywhere)

Example tools:
    node        Node.js JavaScript runtime (npm, npx will also work)
    java        Java runtime and development kit
    gradle      Gradle build automation tool
    go          Go programming language
    flutter     Flutter SDK (dart will also work)

Run 'gg tools' to see all available tools with descriptions

GitHub repos can be accessed directly:
    gh/<owner>/<repo>    Any GitHub release (e.g. gh/cli/cli)

Available tags by tools:
    java: +jdk, +jre, +lts, +sts, +mts, +ea, +ga, +headless, +headfull, +fx, +normal, +hotspot (defaults: +jdk, +ga)
    node: +lts
    go: +beta (excluded by default)
    openapi: +beta (excluded by default)
"
    );
}

fn print_tools() {
    println!("Available tools in gg:\n");

    let tools = get_all_tools();

    let mut languages = Vec::new();
    let mut build_tools = Vec::new();
    let mut utilities = Vec::new();
    let mut github_releases = Vec::new();

    for tool in tools {
        match tool.category {
            ToolCategory::Language => languages.push(tool),
            ToolCategory::BuildTool => build_tools.push(tool),
            ToolCategory::Utility => utilities.push(tool),
            ToolCategory::GitHubRelease => github_releases.push(tool),
        }
    }

    if !languages.is_empty() {
        println!("Languages:");
        for tool in languages {
            print_tool_info(tool);
        }
        println!();
    }

    if !build_tools.is_empty() {
        println!("Build Tools:");
        for tool in build_tools {
            print_tool_info(tool);
        }
        println!();
    }

    if !utilities.is_empty() {
        println!("Utilities:");
        for tool in utilities {
            print_tool_info(tool);
        }
        println!();
    }

    if !github_releases.is_empty() {
        println!("GitHub Releases:");
        for tool in github_releases {
            print_tool_info(tool);
        }
        println!();
    }

    println!("GitHub repos can be accessed directly:");
    println!("    gh/<owner>/<repo>    Any GitHub release (e.g. gh/cli/cli)");
    println!("\nFor more information about a specific tool, use 'gg tools <tool_name>'");
}

fn print_tool_info(tool: &tools::ToolInfo) {
    let aliases = if !tool.aliases.is_empty() {
        format!(" (aliases: {})", tool.aliases.join(", "))
    } else {
        String::new()
    };

    println!("    {:<15} {}{}", tool.name, tool.description, aliases);

    if !tool.tags.is_empty() {
        println!("                    Tags: {}", tool.tags.join(", "));
    }

    if let Some(example) = tool.example {
        println!("                    Example: {}", example);
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let ver = option_env!("VERSION").unwrap_or("dev");

    let cli = Cli::parse();
    let log_level = match cli.get_log_level().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        _ => LevelFilter::Warn,
    };
    if cli.log_external {
        env_logger::builder().filter_level(log_level).init();
    } else {
        env_logger::builder()
            .filter_module("gg", log_level)
            .filter_module("gg:executors", log_level)
            .filter_module("stage4", log_level)
            .filter_module("stage4:executors", log_level)
            .init();
    }

    let cache_base_dir = env::var("GG_CACHE_DIR").unwrap_or_else(|_| {
        if cli.local_cache {
            ".cache/gg".to_string()
        } else {
            let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.cache/gg", home_dir)
        }
    });

    env::set_var("GG_CACHE_DIR", &cache_base_dir);
    info!("Using cache directory: {}", cache_base_dir);
    let system = fs::read_to_string(format!("{}/gg-{ver}/system", cache_base_dir))
        .unwrap_or(String::from("x86_64-linux"))
        .trim()
        .to_string();
    let target =
        Target::parse_with_overrides(&system, cli.override_os.clone(), cli.override_arch.clone());

    let (cmds, app_args) = cli.parse_args();

    let input = &AppInput {
        target,
        app_args: app_args.clone(),
    };

    if cli.version {
        println!("{}", ver);
        return ExitCode::from(0);
    }

    if cli.help {
        print_help(ver);
        return ExitCode::from(0);
    }

    debug!(target: "main", "{:?}", &cli);

    if let Some(cmd) = cmds.first() {
        match cmd.cmd.as_str() {
            "update" => {
                let tool_name = app_args.first().cloned();
                let should_update = cli.get_update_flag();
                let allow_major = cli.get_major_flag();
                let force = cli.get_force_flag();

                match tool_name.as_deref() {
                    None => {
                        checker::check_or_update_all_including_gg(
                            input,
                            ver,
                            should_update,
                            allow_major,
                            force,
                        )
                        .await;
                    }
                    Some("gg") | Some("gg.cmd") => {
                        if should_update {
                            updater::perform_update(ver, force).await;
                        } else {
                            updater::check_gg_update(ver).await;
                        }
                    }
                    Some(tool) => {
                        checker::check_or_update_tool(
                            input,
                            tool,
                            should_update,
                            allow_major,
                            force,
                        )
                        .await;
                    }
                }
                return ExitCode::from(0);
            }
            "tools" => {
                if let Some(tool_name) = app_args.first() {
                    if let Some(tool) = get_tool_info(tool_name) {
                        println!("Tool: {}", tool.name);
                        println!("Description: {}", tool.description);
                        if !tool.aliases.is_empty() {
                            println!("Aliases: {}", tool.aliases.join(", "));
                        }
                        if !tool.tags.is_empty() {
                            println!("Available tags: {}", tool.tags.join(", "));
                        }
                        if let Some(example) = tool.example {
                            println!("Example: {}", example);
                        }
                    } else {
                        println!("Tool '{}' not found", tool_name);
                        println!("\nRun 'gg tools' to see all available tools");
                    }
                } else {
                    print_tools();
                }
                return ExitCode::from(0);
            }
            "clean-cache" => {
                if let Err(e) = cleaner::clean_cache() {
                    println!("Error: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::from(0);
            }
            _ => {}
        };
    }

    let override_info = match (&cli.override_os, &cli.override_arch) {
        (Some(os), Some(arch)) => format!(" (overridden: OS={}, Arch={})", os, arch),
        (Some(os), None) => format!(" (overridden: OS={})", os),
        (None, Some(arch)) => format!(" (overridden: Arch={})", arch),
        (None, None) => String::new(),
    };
    info!("System is {system}{}. {:?}", override_info, &target);

    if cmds.first().is_some() {
        let mut executors = cmds
            .iter()
            .filter_map(|cmd| {
                <dyn Executor>::new(ExecutorCmd {
                    cmd: cmd.cmd.to_string(),
                    version: GgVersionReq::new(
                        cmd.version.clone().unwrap_or("".to_string()).as_str(),
                    ),
                    distribution: cmd.distribution.clone(),
                    include_tags: cmd.include_tags.clone(),
                    exclude_tags: cmd.exclude_tags.clone(),
                })
            })
            .collect::<Vec<Box<dyn Executor>>>();

        let mut look_for_deps = true;
        let mut processed_deps = std::collections::HashSet::new();
        while look_for_deps {
            look_for_deps = false;
            let mut to_add = Vec::new();
            for x in &executors {
                let deps = x.get_deps(input).await;
                for dep in deps {
                    if !executors
                        .iter()
                        .any(|e| &e.get_name().to_string() == &dep.name)
                        && !to_add
                            .iter()
                            .any(|e: &Box<dyn Executor>| &e.get_name().to_string() == &dep.name)
                        && !processed_deps.contains(&dep.name)
                    {
                        if dep.optional {
                            if let Ok(_) = which::which(&dep.name) {
                                info!(
                                    "Optional dependency '{}' found in PATH, using system version",
                                    dep.name
                                );
                                processed_deps.insert(dep.name.clone());
                                continue;
                            } else {
                                info!(
                                    "Optional dependency '{}' not found in PATH, falling back to managed version",
                                    dep.name
                                );
                            }
                        }

                        if let Some(e) = <dyn Executor>::new(ExecutorCmd {
                            cmd: dep.name.clone(),
                            version: dep.version.as_ref().and_then(|v| GgVersionReq::new(v)),
                            distribution: None,
                            include_tags: Default::default(),
                            exclude_tags: Default::default(),
                        }) {
                            look_for_deps = true;
                            processed_deps.insert(dep.name.clone());
                            to_add.push(e);
                        }
                    }
                }
            }
            for x in to_add {
                executors.push(x);
            }
        }

        if executors.first().is_some() {
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
                ExitCode::from(1)
            }
        } else {
            println!(
                "Error: Command not supported. Run './gg.cmd help' to see available commands."
            );
            ExitCode::from(1)
        }
    } else {
        print_help(ver);
        ExitCode::from(0)
    }
}
