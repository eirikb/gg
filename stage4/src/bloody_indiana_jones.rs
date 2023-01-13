use std::cmp::min;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};

pub async fn download_unpack_and_all_that_stuff(url: &str, path: &str) {
    info!("Downloading {url}");

    let client = reqwest::Client::new();
    let res = client.get(url)
        .send()
        .await
        .expect(format!("Failed to get {url}").as_str());
    let total_size = res
        .content_length()
        .expect(format!("Failed to get content length from {url}").as_str());

    debug!("Total size {:?}", total_size);

    create_dir_all(".cache/gg/downloads").expect("Unable to create download dir");

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap());
    pb.set_message(format!("Downloading {}", url));

    let mut file_name = reqwest::Url::parse(url).unwrap().path_segments().unwrap().last().unwrap().to_string();
    debug!("File name {:?}", file_name);

    let file_path = &format!(".cache/gg/downloads/{file_name}");
    debug!("{:?}", file_path);

    let mut file = File::create(file_path)
        .expect(format!("Failed to create file '{}'", file_path).as_str());
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.expect(format!("Error while downloading file").as_str());
        file.write_all(&chunk)
            .expect("Error while writing to file");
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, file_path));

    info!("Extracting {file_name}");
    let ext = Path::new(&file_name).extension().unwrap().to_str();
    let file_buf_reader = tokio::io::BufReader::new(tokio::fs::File::open(file_path).await.unwrap());
    let file_path_decomp = &Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();
    let mut file_writer = tokio::io::BufWriter::new(tokio::fs::File::create(file_path_decomp).await.unwrap());

    match ext {
        Some("xz") | Some("gz") => {
            match ext {
                Some("xz") => {
                    info!("Decompressing Xz");
                    let mut decoder = async_compression::tokio::bufread::XzDecoder::new(file_buf_reader);
                    tokio::io::copy(&mut decoder, &mut file_writer).await.unwrap();
                }
                _ => {
                    info!("Decompressing Gzip");
                    let mut decoder = async_compression::tokio::bufread::GzipDecoder::new(file_buf_reader);
                    tokio::io::copy(&mut decoder, &mut file_writer).await.unwrap();
                }
            };
        }
        Some("zip") => {
            info!("Decompressing Zip");
            debug!("Path is {}", &path);
            let part = path.split("/").last().unwrap_or("unknown");
            let part_path = format!(".cache/{part}/{part}");
            debug!("path_path {}", &part_path);
            create_dir_all(&part_path).expect("Unable to create download dir");
            let target_dir = PathBuf::from(&part_path);
            zip_extract::extract(File::open(file_path).unwrap(), &target_dir, true).unwrap();
        }
        _ => ()
    }

    file_name = Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();

    match Path::new(&file_name).extension().unwrap().to_str() {
        Some("tar") => {
            info!("Untar {file_name}");
            let mut archive = tar::Archive::new(std::io::BufReader::new(File::open(file_name).unwrap()));
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
