use std::{env, fs};

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::executor::{Executor, prep, try_execute};
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
                let input = (target, v.to_string());
                if v == "node" || v == "npm" || v == "npx" {
                    try_execute(&Node {}, input).await.expect("Node: Oh no");
                } else if v == "gradle" {
                    try_execute(&Gradle {}, input).await.expect("Gradle: Oh no");
                } else if v == "java" {
                    try_execute(&Java {}, input).await.expect("Java: Oh no");
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
