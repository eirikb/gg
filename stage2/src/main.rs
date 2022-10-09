use std::{env, fs, io};
use std::fs::{create_dir, File};
use std::io::{Read, Write};
use std::path::Path;

use reqwest;
use reqwest::Url;
use scraper::{Html, Selector};
use crate::bloody_indiana_jones::download_unpack_and_all_that_stuff;
use crate::node::get_node_url;

mod target;
mod bloody_indiana_jones;
mod node;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = target::parse_target(&system);
    println!("target arch {} os {}", target.arch, target.os);

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" {
                    let node_url = get_node_url(&target).await;
                    println!("Node download url: {}", node_url);
                    download_unpack_and_all_that_stuff(&node_url).await;
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