use std::{env, fs};
use std::path::Path;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;

use node::get_node_url;
use bloody_indiana_jones::download_unpack_and_all_that_stuff;
use gradle::get_gradle_url;

fn try_run(path: &str, bin: &str) -> Option<()> {
    println!("Find {bin} in {path}");
    let dir = Path::new(".cache").join(path).read_dir().ok()?.next()?;
    match dir {
        Ok(d) => {
            let dp = &d.path();
            let bin_path = dp.join(bin);
            if bin_path.exists() {
                println!("Executing: {:?}", bin_path);
                let buf = env::current_dir().unwrap();
                let current = buf.to_str().unwrap();
                let bin_path_string = dp.to_str().unwrap_or("");
                let path_string = &env::var("PATH").unwrap_or("".to_string());
                println!("PATH: {current}/{bin_path_string}/bin:{path_string}");
                std::process::Command::new(&bin_path)
                    .env("PATH", format!("{current}/{bin_path_string}/bin:{path_string}"))
                    .args(env::args().skip(2))
                    .spawn().unwrap().wait().unwrap();
                Some(())
            } else {
                println!("Executable not found");
                None
            }
        }
        _ => {
            println!("Cache dir for {path} not found");
            None
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("x86_64-linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = target::parse_target(&system);
    dbg!(&target);

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" || v == "npm" || v == "npx" {
                    let bin = match &target.os {
                        target::Os::Windows => match v.as_str() {
                            "node" => "node.exe",
                            "npm" => "npm.cmd",
                            _ => "npx.cmd",
                        },
                        _ => match v.as_str() {
                            "node" => "bin/node",
                            "npm" => "bin/npm",
                            _ => "bin/npx"
                        }
                    };
                    match try_run("node", bin) {
                        Some(()) => {}
                        None => {
                            println!("NO!");
                            let node_url = get_node_url(&target).await;
                            println!("Node download url: {}", node_url);
                            download_unpack_and_all_that_stuff(&node_url, ".cache/node").await;
                            try_run("node", bin).expect("Unable to execute");
                        }
                    }
                } else if v == "gradle" {
                    println!("Finally - the important bits!");
                    let bin = match &target.os {
                        target::Os::Windows => "gradle.exe",
                        _ => "gradle"
                    };
                    match try_run("gradle", bin) {
                        Some(()) => {}
                        None => {
                            println!("NO!");
                            let gradle_url = get_gradle_url(&target).await;
                            println!("Gradle download url: {}", gradle_url);
                            download_unpack_and_all_that_stuff(&gradle_url, ".cache/gradle").await;
                            try_run("gradle", bin).expect("Unable to execute");
                        }
                    }
                } else {
                    println!("It is {}", v);
                }
            }
            None => {
                println!("Nope");
            }
        }
    }.await;
    // println!("CWD is {}", env::current_dir().unwrap().display())
    // let app = App::new("m")
    //     .version("1.0")
    //     .author("Eirik Brandtzæg. <eirikb@eirikb.no>")
    //     .about("Bootstrap")
    //     .subcommand(SubCommand::with_name("node")
    //         .about("Ugh node"))
    //     .subcommand(SubCommand::with_name("")
    //         .about("Ugh no"));
    // let matches = app.get_matches();
    //
    // let val = matches.value_of("node").unwrap_or("OK");
}