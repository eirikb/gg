use std::cmp::min;
use std::fs::{create_dir_all, read_dir, remove_dir, rename, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use indicatif::ProgressBar;
use log::{debug, info};
use tokio::task;

fn get_file_name(url: &str) -> String {
    reqwest::Url::parse(url)
        .unwrap()
        .path_segments()
        .unwrap()
        .last()
        .unwrap()
        .to_string()
}

const DOWNLOADS_DIR: &str = ".cache/gg/downloads";

pub struct BloodyIndianaJones {
    url: String,
    path: String,
    file_name: String,
    pub file_path: String,
    pb: ProgressBar,
}

impl BloodyIndianaJones {
    pub fn new(url: String, path: String, pb: ProgressBar) -> Self {
        let file_name = get_file_name(&url);
        let file_path = format!("{DOWNLOADS_DIR}/{file_name}");
        Self {
            url,
            path,
            file_name,
            file_path,
            pb,
        }
    }

    pub fn new_with_file_name(url: String, path: String, pb: ProgressBar) -> Self {
        let file_name = get_file_name(&url);
        let file_path = path.clone();
        Self {
            url,
            path,
            file_name,
            file_path,
            pb,
        }
    }

    pub async fn download(&self) {
        info!("Downloading {}", &self.url);
        self.pb.reset();
        self.pb.set_message("Preparing");

        create_dir_all(DOWNLOADS_DIR).expect("Unable to create download dir");

        self.pb.set_message("Downloading");
        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to create HTTP client");
        let res = client
            .get(&self.url)
            .send()
            .await
            .expect(format!("Failed to get {}", &self.url).as_str());
        let total_size = res
            .content_length()
            .expect(format!("Failed to get content length from {}", &self.url).as_str());

        debug!("Total size {:?}", total_size);

        self.pb.set_length(total_size);

        let file_name = get_file_name(&self.url);
        debug!("File name {:?}", file_name);

        debug!("{:?}", &self.file_path);

        let mut file = File::create(&self.file_path)
            .expect(format!("Failed to create file '{}'", &self.file_path).as_str());
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.expect(format!("Error while downloading file").as_str());
            file.write_all(&chunk).expect("Error while writing to file");
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            self.pb.set_position(new);
        }

        info!("Downloaded {} to {}", &self.url, &self.file_path);
    }

    pub async fn unpack_and_all_that_stuff(&self) {
        self.pb.reset();
        self.pb.set_message("Extracting");

        info!("Extracting {}", self.file_name);
        let ext = Path::new(&self.file_name).extension().unwrap().to_str();
        let file_buf_reader =
            tokio::io::BufReader::new(tokio::fs::File::open(&self.file_path).await.unwrap());
        let file_path_decomp = if ext == Some("tgz") {
            // For .tgz files, replace extension with .tar
            &Path::new(&format!("{DOWNLOADS_DIR}/{}", self.file_name))
                .with_extension("tar")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            &Path::new(&format!("{DOWNLOADS_DIR}/{}", self.file_name))
                .with_extension("")
                .to_str()
                .unwrap()
                .to_string()
        };

        match ext {
            Some("xz") | Some("gz") | Some("tgz") => {
                match ext {
                    Some("xz") => {
                        info!("Decompressing Xz");
                        let mut decoder =
                            async_compression::tokio::bufread::XzDecoder::new(file_buf_reader);
                        let mut file_writer = tokio::io::BufWriter::new(
                            tokio::fs::File::create(file_path_decomp).await.unwrap(),
                        );
                        tokio::io::copy(&mut decoder, &mut file_writer)
                            .await
                            .unwrap();
                    }
                    _ => {
                        info!("Decompressing Gzip");
                        self.pb.set_message("Gunzip");
                        let mut decoder =
                            async_compression::tokio::bufread::GzipDecoder::new(file_buf_reader);
                        let mut file_writer = tokio::io::BufWriter::new(
                            tokio::fs::File::create(file_path_decomp).await.unwrap(),
                        );
                        tokio::io::copy(&mut decoder, &mut file_writer)
                            .await
                            .unwrap();
                    }
                };
            }
            Some("zip") => {
                info!("Decompressing Zip");
                info!("Path is {}", &self.path);
                self.pb.set_message("Unzip");
                let file_path_string = self.file_path.clone();
                let path_string = self.path.clone();
                task::spawn_blocking(move || {
                    create_dir_all(&path_string).expect("Unable to create download dir");
                    let target_dir = PathBuf::from(&path_string);
                    zip_extract::extract(File::open(file_path_string).unwrap(), &target_dir, true)
                        .unwrap();
                })
                .await
                .expect("Unable to unzip");
            }
            Some("tar") => (),
            _ => {
                self.pb.set_message("Move");
                create_dir_all(&self.path).expect("Unable to create download dir");
                rename(
                    &self.file_path,
                    self.path.to_string() + "/" + self.file_name.as_str(),
                )
                .unwrap();
                self.pb.finish_with_message("Done");
                return;
            }
        }

        let file_name = if ext == Some("tgz") {
            Path::new(&format!(".cache/gg/downloads/{}", self.file_name))
                .with_extension("tar")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            Path::new(&format!(".cache/gg/downloads/{}", self.file_name))
                .with_extension("")
                .to_str()
                .unwrap()
                .to_string()
        };

        if let Some(extension) = Path::new(&file_name).extension() {
            if extension == "tar" {
                info!("Untar {file_name}");
                self.pb.set_message("Untar");
                let mut archive =
                    tar::Archive::new(std::io::BufReader::new(File::open(file_name).unwrap()));
                archive.unpack(&self.path).expect("Unable to extract");
            }
        }

        let path_string = self.path.clone();
        self.pb.set_message("Move");
        task::spawn_blocking(move || {
            let parent_path = Path::new(&path_string);
            let entries = read_dir(&path_string);
            if let Ok(entries) = entries {
                let entries = entries.collect::<Vec<_>>();
                if entries.len() == 1 {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if entry.path().is_dir() {
                                debug!(
                                    "Extracted files are contained in sub-folder. Moving them up"
                                );
                                let parent = entry.path();
                                if let Ok(entries) = read_dir(&parent) {
                                    for entry in entries {
                                        if let Ok(entry) = entry {
                                            let path = entry.path();
                                            let new_path =
                                                parent_path.join(path.file_name().unwrap());
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
        })
        .await
        .expect("Unable to move files");
        self.pb.finish_with_message("Done");
    }
}
