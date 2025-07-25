use std::cmp::min;
use std::env::temp_dir;
use std::fs::{create_dir_all, read_dir, remove_dir, remove_file, rename, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::StreamExt;
use indicatif::ProgressBar;
use log::{debug, info};
use tokio::io::{AsyncRead, ReadBuf};
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

struct ProgressReader<R> {
    inner: R,
    progress_bar: ProgressBar,
    bytes_read: u64,
}

impl<R> ProgressReader<R> {
    fn new(inner: R, progress_bar: ProgressBar) -> Self {
        Self {
            inner,
            progress_bar,
            bytes_read: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);

        if let Poll::Ready(Ok(())) = result {
            let bytes_read = buf.filled().len() - before;
            self.bytes_read += bytes_read as u64;
            self.progress_bar.set_position(self.bytes_read);
        }

        result
    }
}

pub struct BloodyIndianaJones {
    url: String,
    path: String,
    file_name: String,
    pub file_path: String,
    pb: ProgressBar,
    temp_dir: PathBuf,
}

impl BloodyIndianaJones {
    pub fn new_with_cache_dir(
        url: String,
        path: String,
        cache_base_dir: &str,
        pb: ProgressBar,
    ) -> Self {
        let file_name = get_file_name(&url);
        let file_path = format!("{cache_base_dir}/downloads/{file_name}");
        let temp_dir = temp_dir().join(format!("gg_process_{}", std::process::id()));
        info!("BloodyIndianaJones downloads path: {}", file_path);
        info!("BloodyIndianaJones temp directory: {}", temp_dir.display());
        Self {
            url,
            path,
            file_name,
            file_path,
            pb,
            temp_dir,
        }
    }

    pub fn new_with_file_name(url: String, path: String, pb: ProgressBar) -> Self {
        let file_name = get_file_name(&url);
        let file_path = path.clone();
        let temp_dir = temp_dir().join(format!("gg_process_{}", std::process::id()));
        Self {
            url,
            path,
            file_name,
            file_path,
            pb,
            temp_dir,
        }
    }

    pub async fn download(&self) {
        info!("Downloading {}", &self.url);
        self.pb.reset();
        self.pb.set_message("Preparing");

        if let Some(parent) = Path::new(&self.file_path).parent() {
            create_dir_all(parent).expect("Unable to create download dir");
        }

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
            let chunk = item.expect("Error while downloading file".to_string().as_str());
            file.write_all(&chunk).expect("Error while writing to file");
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            self.pb.set_position(new);
        }

        info!("Downloaded {} to {}", &self.url, &self.file_path);
    }

    pub async fn unpack_and_all_that_stuff(&mut self) {
        self.pb.reset();
        self.pb.set_message("Extracting");

        info!("Extracting {}", self.file_name);
        let ext = Path::new(&self.file_name).extension().unwrap().to_str();

        let file_size = std::fs::metadata(&self.file_path).unwrap().len();
        self.pb.set_length(file_size);
        self.pb.set_position(0);

        let file = tokio::fs::File::open(&self.file_path).await.unwrap();
        let progress_reader = ProgressReader::new(file, self.pb.clone());
        let file_buf_reader = tokio::io::BufReader::new(progress_reader);
        create_dir_all(&self.temp_dir).expect("Unable to create temp directory");

        let original_file_name = Path::new(&self.file_path).file_name().unwrap();
        let file_path_decomp = if ext == Some("tgz") {
            self.temp_dir
                .join(original_file_name)
                .with_extension("tar")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            self.temp_dir
                .join(original_file_name)
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
                            tokio::fs::File::create(file_path_decomp.clone())
                                .await
                                .unwrap(),
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
                            tokio::fs::File::create(file_path_decomp.clone())
                                .await
                                .unwrap(),
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
                let pb_clone = self.pb.clone();
                task::spawn_blocking(move || {
                    create_dir_all(&path_string).expect("Unable to create download dir");
                    let target_dir = PathBuf::from(&path_string);
                    zip_extract::extract(File::open(file_path_string).unwrap(), &target_dir, true)
                        .unwrap();
                    pb_clone.set_position(pb_clone.length().unwrap_or(0));
                })
                .await
                .expect("Unable to unzip");
            }
            Some("tar") => (),
            _ => {
                self.pb.set_message("Move");
                self.pb.set_length(1);
                self.pb.set_position(0);
                create_dir_all(&self.path).expect("Unable to create download dir");
                rename(
                    &self.file_path,
                    self.path.to_string() + "/" + self.file_name.as_str(),
                )
                .unwrap();
                self.pb.set_position(1);
                self.pb.finish_with_message("Done");
                println!();
                return;
            }
        }

        let file_name = &file_path_decomp;

        if let Some(extension) = Path::new(&file_name).extension() {
            if extension == "tar" {
                info!("Untar {file_name}");
                self.pb.set_message("Untar");

                let tar_file_size = std::fs::metadata(file_name).unwrap().len();
                self.pb.set_length(tar_file_size);
                self.pb.set_position(0);

                let temp_extract_dir = self.temp_dir.join("extracted");
                create_dir_all(&temp_extract_dir).expect("Unable to create temp extract dir");

                let mut archive =
                    tar::Archive::new(std::io::BufReader::new(File::open(file_name).unwrap()));
                archive
                    .unpack(&temp_extract_dir)
                    .expect("Unable to extract");

                self.pb.set_position(tar_file_size);

                create_dir_all(&self.path).expect("Unable to create final dir");
                self.move_temp_contents_to_final(&temp_extract_dir, Path::new(&self.path));
            }
        }

        let path_string = self.path.clone();
        self.pb.set_message("Move");
        self.pb.set_length(0);
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
        println!();
    }

    fn move_temp_contents_to_final(&self, temp_dir: &Path, final_dir: &Path) {
        if let Ok(entries) = read_dir(temp_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let temp_path = entry.path();
                    let file_name = temp_path.file_name().unwrap();
                    let final_path = final_dir.join(file_name);

                    if let Err(e) = rename(&temp_path, &final_path) {
                        debug!(
                            "Failed to move {} to {}: {}",
                            temp_path.display(),
                            final_path.display(),
                            e
                        );
                    } else {
                        info!("Moved {} to {}", temp_path.display(), final_path.display());
                    }
                }
            }
        }
    }

    pub fn cleanup_download(&self) {
        if Path::new(&self.file_path).exists() {
            info!("Cleaning up downloaded file: {}", &self.file_path);
            if let Err(e) = remove_file(&self.file_path) {
                debug!("Failed to remove download file {}: {}", &self.file_path, e);
            } else {
                info!("Successfully removed download file: {}", &self.file_path);
            }
        }

        if self.temp_dir.exists() {
            info!("Cleaning up temp directory: {}", self.temp_dir.display());
            if let Err(e) = std::fs::remove_dir_all(&self.temp_dir) {
                debug!(
                    "Failed to remove temp directory {}: {}",
                    self.temp_dir.display(),
                    e
                );
            } else {
                info!(
                    "Successfully removed temp directory: {}",
                    self.temp_dir.display()
                );
            }
        }
    }
}
