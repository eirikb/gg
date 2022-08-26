// extern crate clap;

// use clap::{App, SubCommand};

use std::env;
use reqwest;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(v) => {
            if v == "node" {
                println!("GO NODE");
                let body = reqwest::get("https://nodejs.org/en/download/").await.unwrap().text().await.unwrap();
                println!("{}", body);
            } else {
                println!("It is {}", v);
            }
        }
        None => {
            println!("Nope");
        }
    }
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