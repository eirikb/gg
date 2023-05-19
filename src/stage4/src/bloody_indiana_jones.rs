use futures_util::StreamExt;
use indicatif::ProgressBar;
use log::{debug, info};
use std::cmp::min;
use std::fs::{create_dir_all, File, read_dir, remove_dir, rename};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::task;

fn get_file_name(url: &str) -> String {
    reqwest::Url::parse(url).unwrap().path_segments().unwrap().last().unwrap().to_string()
}

pub async fn download(url: &str, file_path: &str, pb: &ProgressBar) {
    pb.set_message("Downloading");
    let client = reqwest::Client::new();
    let res = client.get(url)
        .send()
        .await
        .expect(format!("Failed to get {url}").as_str());
    let total_size = res
        .content_length()
        .expect(format!("Failed to get content length from {url}").as_str());

    debug!("Total size {:?}", total_size);

    pb.set_length(total_size);

    let file_name = get_file_name(url);
    debug!("File name {:?}", file_name);

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

    info!("Downloaded {} to {}", url, file_path);
}

pub async fn download_unpack_and_all_that_stuff(url: &str, path: &str, pb: &ProgressBar) {
    info!("Downloading {url}");
    pb.set_message("Preparing");

    let ver = option_env!("VERSION").unwrap_or("dev");
    let downloads_dir = &format!(".cache/gg-{ver}/downloads");
    create_dir_all(downloads_dir).expect("Unable to create download dir");
    let file_name = get_file_name(url);
    let file_path = &format!("{downloads_dir}/{file_name}");
    download(url, file_path.as_str(), pb).await;
    pb.reset();
    pb.set_message("Extracting");

    info!("Extracting {file_name}");
    let ext = Path::new(&file_name).extension().unwrap().to_str();
    let file_buf_reader = tokio::io::BufReader::new(tokio::fs::File::open(file_path).await.unwrap());
    let file_path_decomp = &Path::new(&format!("{downloads_dir}/{file_name}")).with_extension("").to_str().unwrap().to_string();
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
                    pb.set_message("Gunzip");
                    let mut decoder = async_compression::tokio::bufread::GzipDecoder::new(file_buf_reader);
                    tokio::io::copy(&mut decoder, &mut file_writer).await.unwrap();
                }
            };
        }
        Some("zip") => {
            info!("Decompressing Zip");
            info!("Path is {}", &path);
            pb.set_message("Unzip");
            let file_path_string = String::from(file_path);
            let path_string = String::from(path);
            task::spawn_blocking(move || {
                create_dir_all(&path_string).expect("Unable to create download dir");
                let target_dir = PathBuf::from(&path_string);
                zip_extract::extract(File::open(file_path_string).unwrap(), &target_dir, true).unwrap();
            }).await.expect("Unable to unzip");
        }
        _ => ()
    }

    let file_name = Path::new(&format!(".cache/gg-{ver}/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();

    match Path::new(&file_name).extension().unwrap().to_str() {
        Some("tar") => {
            info!("Untar {file_name}");
            pb.set_message("Untar");
            let mut archive = tar::Archive::new(std::io::BufReader::new(File::open(file_name).unwrap()));
            archive.unpack(path).expect("Unable to extract");
        }
        _ => {}
    }

    let path_string = String::from(path);
    pb.set_message("Move");
    task::spawn_blocking(move || {
        let parent_path = Path::new(&path_string);
        let entries = read_dir(&path_string);
        if let Ok(entries) = entries {
            let entries = entries.collect::<Vec<_>>();
            if entries.len() == 1 {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().is_dir() {
                            debug!("Extracted files are contained in sub-folder. Moving them up");
                            let parent = entry.path();
                            if let Ok(entries) = read_dir(&parent) {
                                for entry in entries {
                                    if let Ok(entry) = entry {
                                        let path = entry.path();
                                        let new_path = parent_path.join(path.file_name().unwrap());
                                        rename(&path, new_path).unwrap();
                                    }
                                }
                                remove_dir(parent).ok();
                            }
                        }
                    }
                }
            }
        }
    }).await.expect("Unable to move files");
    pb.finish_with_message("Done");
}
