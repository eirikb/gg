use std::{env, fs};

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::executor::{Executor, try_execute};
use crate::gradle::Gradle;
use crate::java::Java;
use crate::node::Node;

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
                    try_execute(Box::new(Node {}), target, v.to_string()).await.expect("Node: Oh no");
                } else if v == "gradle" {
                    // prep_java(target).await.expect("Unable to prep Java");
                    // try_run_gradle(target).await.expect("Gradle fail!");
                    try_execute(Box::new(Gradle {}), target, v.to_string()).await.expect("Gradle: Oh no");
                } else if v == "java" {
                    try_execute(Box::new(Java {}), target, v.to_string()).await.expect("Java: Oh no");
                } else {
                    println!("It is {}", v);
                }
            }
            None => {
                println!("Nope");
            }
        }
    }.await;
}
