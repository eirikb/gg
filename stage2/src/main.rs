use std::{env, fs};
use std::path::Path;

mod target;
mod bloody_indiana_jones;
mod node;

use node::get_node_url;
use bloody_indiana_jones::download_unpack_and_all_that_stuff;

async fn run(url: &String, path: &String) {
    let cache_path = String::from(".cache/") + path;
    if !Path::new(&cache_path).exists() {
        println!("{path} not found, installing...");
        download_unpack_and_all_that_stuff(&url, &cache_path).await;
    }
    let dir = std::fs::read_dir(&cache_path).unwrap().next()
        .expect("{path} folder not found").expect("");
    println!("Dir is {}", dir.path().to_str().unwrap());

    // let bin = Path::new(".cache").join(path).join("bin").join(path).to_str().unwrap();
    // println!("bin is {bin}");
}

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
                    let node_url = get_node_url(&target).await;
                    println!("Node download url: {}", node_url);
                    run(&node_url, &String::from("node")).await;
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