use std::{env, fs};

use reqwest;
use scraper::{Html, Selector};
use crate::target::target::parse_target;

mod target;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = parse_target(&system);
    if target.arch == target::target::Arch::X86_64 {
        println!("X86!");
    } else {
        println!("Not x86");
    }

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" {
                    println!("GO NODE");
                    let body = reqwest::get("https://nodejs.org/en/download/").await.unwrap().text().await.unwrap();
                    // println!("{}", body);
                    let document = Html::parse_document(body.as_str());
                    let url_selector = Selector::parse(".download-matrix a").unwrap();
                    let node_urls = document.select(&url_selector).map(|x| {
                        x.value().attr("href").unwrap().to_string()
                    }).collect::<Vec<_>>();
                    println!("{}", node_urls.len());

                    for x in node_urls {
                        println!("{}", x);
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