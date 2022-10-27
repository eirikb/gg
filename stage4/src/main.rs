use std::{env, fs};
use std::path::Path;

mod target;
mod bloody_indiana_jones;
mod node;

use node::get_node_url;
use bloody_indiana_jones::download_unpack_and_all_that_stuff;

fn try_run(path: &str, bin: &str) -> Option<()> {
    println!("Find {bin} in {path}");
    let dir = Path::new(".cache").join(path).read_dir().ok()?.next()?;
    match dir {
        Ok(d) => {
            let bin_path = d.path().join(bin);
            if bin_path.exists() {
                println!("Executing: {:?}", bin_path);
                std::process::Command::new(bin_path)
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
    println!("target arch {} os {}", target.arch, target.os);

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" {
                    let bin = match &target.os {
                        target::Os::Windows => "node.exe",
                        _ => "bin/node"
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