use std::{env, fs};
use std::path::Path;

mod target;
mod bloody_indiana_jones;
mod node;

use node::get_node_url;
use bloody_indiana_jones::download_unpack_and_all_that_stuff;

async fn run(url: &String, path: &String) {
    download_unpack_and_all_that_stuff(&url).await;
    if !Path::new(".cache/node").exists() {
        println!("Node not found, installing...");
        download_unpack_and_all_that_stuff(&url).await;
    }
    let dir = std::fs::read_dir(".cache/node").unwrap().next()
        .expect("node folder not found").expect("");
    println!("Dir is {}", dir.path().to_str().unwrap());
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