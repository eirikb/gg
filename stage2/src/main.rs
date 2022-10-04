use std::{env, fs, io};
use std::fs::{create_dir, File};
use std::io::{Read, Write};
use std::path::Path;

use reqwest;
use reqwest::Url;
use scraper::{Html, Selector};

use crate::target::target::parse_target;

mod target;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let system = fs::read_to_string(".cache/gg/system").unwrap_or(String::from("linux")).trim().to_string();
    println!("System is {:?}", system);
    let target = parse_target(&system);
    println!("target arch {} os {}", target.arch, target.os);

    async {
        match args.get(1) {
            Some(v) => {
                if v == "node" {
                    let body = reqwest::get("https://nodejs.org/en/download/").await
                        .expect("Unable to connect to nodejs.org").text().await
                        .expect("Unable to download nodejs list of versions");

                    let document = Html::parse_document(body.as_str());
                    let url_selector = Selector::parse(".download-matrix a")
                        .expect("Unable to find nodejs version to download");

                    let node_urls = document.select(&url_selector).map(|x| {
                        x.value().attr("href")
                            .expect("Unable to find link to nodejs download").to_string()
                    }).collect::<Vec<_>>();

                    for x in &node_urls {
                        println!("{}", x);
                    }
                    let node_url = node_urls.into_iter().filter(|url|
                        match &target.arch {
                            X86_64 => url.contains("x64"),
                            Armv7 => url.contains("armv7l")
                        }
                    ).find(|url|
                        match &target.os {
                            Linux => url.contains("linux") && url.contains(".tar.xz"),
                            Windows => url.contains("win") && url.contains(".zip"),
                            Mac => url.contains("darwin") && url.contains(".tar.xz")
                        }
                    ).expect("Unable to find matching nodejs version against your arch/os");
                    println!("URL is {node_url}");

                    let file_name = Url::parse(&node_url).unwrap().path_segments().unwrap().last().unwrap().to_string();

                    println!("Downloading {node_url}");
                    let res = reqwest::get(node_url).await
                        .expect("Unable to download nodejs");
                    create_dir(".cache/gg/downloads");

                    let file_path = &format!(".cache/gg/downloads/{file_name}");
                    let mut out = File::create(file_path)
                        .expect("Unable to create nodejs archive file");
                    let bytes = res.bytes().await.expect("doug");

                    io::copy(&mut bytes.as_ref(), &mut out)
                        .expect("Unable to download the file?!");

                    println!("Extracting {file_name}");
                    // let mut f = io::BufReader::new(File::open(file_path).unwrap());
                    // let mut decomp: Vec<u8> = Vec::new();
                    // lzma_rs::xz_decompress(&mut f, &mut decomp).unwrap();
                    // io::copy(&mut f, &mut decomp)
                    //     .expect("Unable to download the file?!");
                    //
                    // let file_path_decomp = Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();
                    // println!("Write to {file_path_decomp}");
                    // let mut archive = tar::Archive::


                    // let mut f2 = io::BufWriter::new(File::open(file_path_decomp).unwrap());
                    // f2.write(&decomp);
                    println!("DONE!");
                    // let mut out2 = File::create(format!(".cache/gg/downloads/{file_name}.wat"))
                    //     .expect("Unable to create nodejs archive file");
                    // out2.write(decomp.as_slice());
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