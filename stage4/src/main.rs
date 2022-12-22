use std::{env, fs};

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::gradle::try_run_gradle;
use crate::java::{prep_java, try_run_java};
use crate::node::try_run_node;

mod target;
mod bloody_indiana_jones;
mod node;
mod gradle;
mod java;
mod executor;

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
                    {
                        try_run_node(target, v).await.expect("NODE fail");
                    }
                } else if v == "gradle" {
                    prep_java(target).await.expect("Unable to prep Java");
                    try_run_gradle(target).await.expect("Gradle fail!");
                } else if v == "java" {
                    try_run_java(target).await.expect("Java fail");
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
