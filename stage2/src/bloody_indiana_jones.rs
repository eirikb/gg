use std::fs::{create_dir_all, File};
use std::path::Path;

pub async fn download_unpack_and_all_that_stuff(url: &String, path: &String) {
    println!("Downloading {url}");
    let res = reqwest::get(url).await
        .expect("Unable to download");
    create_dir_all(".cache/gg/downloads").expect("Unable to create download dir");

    let mut file_name = reqwest::Url::parse(url).unwrap().path_segments().unwrap().last().unwrap().to_string();

    let file_path = &format!(".cache/gg/downloads/{file_name}");
    let mut out = File::create(file_path)
        .expect("Unable to create archive file");
    let bytes = res.bytes().await.expect("duh");

    std::io::copy(&mut bytes.as_ref(), &mut out)
        .expect("Unable to download the file?!");
    println!("Done...");

    println!("Extracting {file_name}");
    let ext = Path::new(&file_name).extension().unwrap().to_str();
    let file_buf_reader = tokio::io::BufReader::new(tokio::fs::File::open(file_path).await.unwrap());
    let file_path_decomp = &Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();
    let mut file_writer = tokio::io::BufWriter::new(tokio::fs::File::create(file_path_decomp).await.unwrap());

    match ext {
        Some("xz") | Some("gz") => {
            match ext {
                Some("xz") => {
                    let mut decoder = async_compression::tokio::bufread::XzDecoder::new(file_buf_reader);
                    tokio::io::copy(&mut decoder, &mut file_writer).await.unwrap();
                }
                _ => {
                    let mut decoder = async_compression::tokio::bufread::GzipDecoder::new(file_buf_reader);
                    tokio::io::copy(&mut decoder, &mut file_writer).await.unwrap();
                }
            };
        }
        Some("zip") => {
            // let mut decoder = async_compression::tokio::bufread::DeflateDecoder::new(file_buf_reader);
            // decoder.
            //     tokio
            // ::io::copy(&mut decoder, &mut file_writer).await.unwrap();
        }
        _ => ()
    }

    file_name = Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();

    match Path::new(&file_name).extension().unwrap().to_str() {
        Some("tar") => {
            println!("Untar {file_name}");
            let mut archive = tar::Archive::new(std::io::BufReader::new(std::fs::File::open(file_name).unwrap()));
            archive.unpack(path).expect("Unable to extract");
        }
        _ => {}
    }
}

#[cfg(test)]
mod test {
    use crate::bloody_indiana_jones::download_unpack_and_all_that_stuff;

    #[tokio::test]
    async fn ok() {
        let url = String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz");
        download_unpack_and_all_that_stuff(&url, &String::from("node")).await;
    }
}