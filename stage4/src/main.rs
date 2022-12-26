use std::{env, fs};

use bloody_indiana_jones::download_unpack_and_all_that_stuff;

use crate::executor::{AppInput, Executor, prep, try_execute};
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

    match args.get(1) {
        Some(cmd) => {
            let executor: Option<&dyn Executor> = match cmd.as_str() {
                "node" | "npm" | "npx" => Some(&Node {}),
                "gradle" => Some(&Gradle {}),
                "java" => Some(&Java {}),
                _ => None
            };
            if executor.is_some() {
                try_execute(executor.unwrap(), AppInput { target, cmd: cmd.to_string() }).await.unwrap();
            } else {
                println!("No such command {cmd}");
            }
        }
        None => {
            println!("No command")
        }
    };
}
