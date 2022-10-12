use std::{env, fs};
use std::path::Path;
use std::process::Stdio;

mod target;
mod bloody_indiana_jones;
mod node;

use node::get_node_url;
use bloody_indiana_jones::download_unpack_and_all_that_stuff;

fn try_run(path: &str, bin: &str) -> Option<()> {
    let dir = Path::new(".cache").join(path).read_dir().ok()?.next()?;
    match dir {
        Ok(d) => {
            let bin_path = d.path().join(bin);
            if bin_path.exists() {
                println!("Ready to execute this");
                println!("{:?}", bin_path);
                std::process::Command::new(bin_path)
                    .stdout(Stdio::inherit())
                    .spawn().unwrap();
                Some(())
            } else {
                None
            }
        }
        _ => None
    }
    // let cache_path = String::from(".cache/") + path;
    // let dir = std::fs::read_dir(&cache_path).ok()?.next()?.ok();
    // match dir {
    //     Some(dir) => {
    //         println!("{:?}", dir.path().as_os_str());
    //         if Path::new(dir.path().as_os_str()).join(path).join(bin).exists() {
    //             Some(())
    //         } else {
    //             None
    //         }
    //     }
    //     _ => None
    // }
}

// async fn run(url: &String, path: &String) {
//     if !Path::new(&cache_path).exists() {
//         println!("{path} not found, installing...");
//         download_unpack_and_all_that_stuff(&url, &cache_path).await;
//     }
//
//     // let bin = Path::new(".cache").join(path).join("bin").join(path).to_str().unwrap();
//     // println!("bin is {bin}");
// }

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("x86_64-linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = target::parse_target(&system);
    println!("target arch {} os {}", target.arch, target.os);

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" {
                    match try_run("node", "bin/node") {
                        Some(()) => {
                            println!("OK!");
                        }
                        None => {
                            println!("NO!");
                            let node_url = get_node_url(&target).await;
                            println!("Node download url: {}", node_url);
                            download_unpack_and_all_that_stuff(&node_url, ".cache/node").await;
                            try_run("node", "bin/node").unwrap();
                            // try_run("node", "bin/node").unwrap();
                        }
                    }
                    println!("DONE!");
                } else {
                    println!("It is {}", v);
                }
            }
            None => {
                println!("Nope");
            }
        }
    }.await;
    println!("CWD is {}", env::current_dir().unwrap().display())
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