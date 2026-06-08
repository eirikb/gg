use std::cmp::min;
use std::fs::{create_dir_all, read_dir, remove_dir, remove_dir_all, rename, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use indicatif::ProgressBar;
use log::{debug, info};
use tempfile::tempdir;
use tokio::task;

use crate::gem_utils;

fn get_file_name(url: &str) -> String {
    reqwest::Url::parse(url)
        .unwrap()
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_string()
}

pub struct BloodyIndianaJones {
    url: String,
    path: String,
    file_name: String,
    pub file_path: String,
    pb: ProgressBar,
    temp_dir: tempfile::TempDir,
}

impl BloodyIndianaJones {
    pub fn new_with_cache_dir(
        url: String,
        path: String,
        _cache_base_dir: &str,
        pb: ProgressBar,
    ) -> Self {
        let file_name = get_file_name(&url);
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let file_path = temp_dir
            .path()
            .join(&file_name)
            .to_string_lossy()
            .to_string();
        info!(
            "BloodyIndianaJones temp directory: {}",
            temp_dir.path().display()
        );
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
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let file_path = temp_dir
            .path()
            .join(&file_name)
            .to_string_lossy()
            .to_string();
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
        self.pb.set_message("Downloading");

        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        // Release CDNs (GitHub, Azul, ...) intermittently return 5xx/429,
        // especially from CI. Retry transient failures with linear backoff so
        // a single hiccup doesn't fail the whole run.
        let max_attempts = 5;
        let mut last_error = String::new();
        for attempt in 1..=max_attempts {
            match self.try_download(&client).await {
                Ok(()) => {
                    info!("Downloaded {} to {}", &self.url, &self.file_path);
                    return;
                }
                Err((reason, retryable)) if retryable && attempt < max_attempts => {
                    let backoff = std::time::Duration::from_secs(attempt as u64);
                    info!(
                        "Download attempt {}/{} failed ({}). Retrying in {}s...",
                        attempt,
                        max_attempts,
                        reason,
                        backoff.as_secs()
                    );
                    last_error = reason;
                    tokio::time::sleep(backoff).await;
                }
                Err((reason, _)) => panic!("Failed to download {}: {}", &self.url, reason),
            }
        }
        panic!("Failed to download {}: {}", &self.url, last_error);
    }

    /// One download attempt. Err carries (reason, retryable).
    async fn try_download(&self, client: &reqwest::Client) -> Result<(), (String, bool)> {
        let res = match client.get(&self.url).send().await {
            Ok(res) => res,
            Err(e) => return Err((format!("request failed: {e}"), true)),
        };

        let status = res.status();
        if !status.is_success() {
            // 5xx and 429 are transient; other 4xx (e.g. 404) are permanent.
            let retryable =
                status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS;
            return Err((format!("server returned HTTP {status}"), retryable));
        }

        let total_size = res
            .content_length()
            .unwrap_or_else(|| panic!("Failed to get content length from {}", &self.url));
        debug!("Total size {:?}", total_size);
        self.pb.set_length(total_size);

        let mut file = File::create(&self.file_path)
            .unwrap_or_else(|_| panic!("Failed to create file '{}'", &self.file_path));
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(e) => return Err((format!("connection dropped: {e}"), true)),
            };
            file.write_all(&chunk).expect("Error while writing to file");
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            self.pb.set_position(new);
        }

        Ok(())
    }

    pub async fn unpack_and_all_that_stuff(&mut self) {
        self.pb.reset();
        self.pb.set_message("Extracting");

        info!("Extracting {}", self.file_name);
        let ext = Path::new(&self.file_name).extension().unwrap().to_str();
        let file_buf_reader =
            tokio::io::BufReader::new(tokio::fs::File::open(&self.file_path).await.unwrap());
        let file_path_decomp = if ext == Some("tgz") {
            self.temp_dir
                .path()
                .join(&self.file_name)
                .with_extension("tar")
                .to_str()
                .unwrap()
                .to_string()
        } else {
            self.temp_dir
                .path()
                .join(&self.file_name)
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
                task::spawn_blocking(move || {
                    create_dir_all(&path_string).expect("Unable to create download dir");
                    let target_dir = PathBuf::from(&path_string);
                    let file = File::open(file_path_string).unwrap();
                    zip::ZipArchive::new(file).unwrap().extract(&target_dir).unwrap();
                })
                .await
                .expect("Unable to unzip");
            }
            Some("7z") => {
                info!("Decompressing 7z");
                info!("Path is {}", &self.path);
                self.pb.set_message("Un7z");
                let file_path_string = self.file_path.clone();
                let path_string = self.path.clone();
                task::spawn_blocking(move || {
                    create_dir_all(&path_string).expect("Unable to create download dir");
                    let archive_file = File::open(file_path_string).unwrap();
                    sevenz_rust::decompress(archive_file, &path_string).unwrap();
                })
                .await
                .expect("Unable to extract 7z");
            }
            Some("tar") => (),
            Some("gem") => {
                info!("Processing gem file");
                self.pb.set_message("Installing gem");
                create_dir_all(&self.path).expect("Unable to create download dir");

                let gem_path = Path::new(&self.path).join(&self.file_name);
                std::fs::copy(&self.file_path, &gem_path).unwrap();

                let _ = gem_utils::install_gem_to_cache(gem_path.to_str().unwrap(), &self.path);
            }
            _ => {
                self.pb.set_message("Copy");
                create_dir_all(&self.path).expect("Unable to create download dir");
                std::fs::copy(&self.file_path, Path::new(&self.path).join(&self.file_name))
                    .unwrap();
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
                    for entry in entries.into_iter().flatten() {
                        if entry.path().is_dir() {
                            debug!(
                                "Extracted files are contained in sub-folder. Moving them up"
                            );
                            let parent = entry.path();
                            if let Ok(entries) = read_dir(&parent) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    let new_path =
                                        parent_path.join(path.file_name().unwrap());
                                    rename(&path, new_path).unwrap();
                                }
                                remove_dir(parent).ok();
                            }
                        }
                    }
                }
            }

            // macOS JDK bundles (e.g. zulu26+) nest everything under Contents/Home.
            // Move the real home up so bin/ etc. sit directly in the install dir
            let contents_home = parent_path.join("Contents").join("Home");
            if contents_home.is_dir() {
                debug!("Found macOS bundle layout (Contents/Home). Moving them up");
                if let Ok(entries) = read_dir(&contents_home) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let new_path = parent_path.join(path.file_name().unwrap());
                        rename(&path, new_path).unwrap();
                    }
                }
                remove_dir_all(parent_path.join("Contents")).ok();
            }
        })
        .await
        .expect("Unable to move files");
        self.pb.finish_with_message("Done");
        println!();
    }

    pub fn cleanup_download(&self) {
        info!(
            "Cleaning up temp directory: {}",
            self.temp_dir.path().display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_tar_gz(file_path: &str, entries: &[&str]) {
        let mut builder = tar::Builder::new(Vec::new());
        for entry in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(0);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, entry, std::io::empty())
                .unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();
        let mut encoder = async_compression::tokio::write::GzipEncoder::new(
            tokio::fs::File::create(file_path).await.unwrap(),
        );
        tokio::io::AsyncWriteExt::write_all(&mut encoder, &tar_bytes)
            .await
            .unwrap();
        tokio::io::AsyncWriteExt::shutdown(&mut encoder)
            .await
            .unwrap();
    }

    async fn unpack(entries: &[&str]) -> tempfile::TempDir {
        let target = tempdir().unwrap();
        let path = target.path().join("java_star_");
        let mut bij = BloodyIndianaJones::new_with_file_name(
            "http://example.com/test.tar.gz".to_string(),
            path.to_str().unwrap().to_string(),
            ProgressBar::hidden(),
        );
        make_tar_gz(&bij.file_path, entries).await;
        bij.unpack_and_all_that_stuff().await;
        target
    }

    #[tokio::test]
    async fn test_unpack_macos_jdk_bundle_layout() {
        // zulu26+ macOS tarballs nest everything under <top>/Contents/Home
        let target = unpack(&[
            "zulu26.30.11-ca-jdk26.0.1-macosx_x64/Contents/Home/bin/java",
            "zulu26.30.11-ca-jdk26.0.1-macosx_x64/Contents/Home/lib/jvm.cfg",
            "zulu26.30.11-ca-jdk26.0.1-macosx_x64/Contents/Info.plist",
        ])
        .await;
        let install_dir = target.path().join("java_star_");
        assert!(install_dir.join("bin").join("java").exists());
        assert!(install_dir.join("lib").join("jvm.cfg").exists());
        assert!(!install_dir.join("Contents").exists());
    }

    #[tokio::test]
    async fn test_unpack_regular_layout() {
        // zulu25-style tarballs have bin/lib directly under the top dir
        let target = unpack(&[
            "zulu25.28.85-ca-jdk25.0.0-macosx_x64/bin/java",
            "zulu25.28.85-ca-jdk25.0.0-macosx_x64/lib/jvm.cfg",
        ])
        .await;
        let install_dir = target.path().join("java_star_");
        assert!(install_dir.join("bin").join("java").exists());
        assert!(install_dir.join("lib").join("jvm.cfg").exists());
    }

    // Serve a sequence of raw HTTP responses, one per incoming connection.
    // Returns the bound port.
    async fn serve_seq(responses: Vec<&'static [u8]>) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            for response in responses {
                if let Ok((mut socket, _)) = listener.accept().await {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let _ = socket.write_all(response).await;
                    let _ = socket.shutdown().await;
                }
            }
        });
        port
    }

    fn bij_for(port: u16, target: &tempfile::TempDir) -> BloodyIndianaJones {
        BloodyIndianaJones::new_with_file_name(
            format!("http://127.0.0.1:{port}/tool.tar.gz"),
            target.path().join("out").to_str().unwrap().to_string(),
            ProgressBar::hidden(),
        )
    }

    #[tokio::test]
    #[should_panic(expected = "server returned HTTP 403")]
    async fn test_download_rejects_permanent_status() {
        // A 4xx (e.g. forbidden) is permanent: fail immediately with a clear
        // message, not stream the error body and panic later as "invalid gzip".
        let port = serve_seq(vec![
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 9\r\n\r\nforbidden",
        ])
        .await;
        let target = tempdir().unwrap();
        bij_for(port, &target).download().await;
    }

    #[tokio::test]
    async fn test_download_retries_transient_status() {
        // A transient 503 should be retried; the next attempt succeeds.
        let port = serve_seq(vec![
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 3\r\n\r\nbad",
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ngood",
        ])
        .await;
        let target = tempdir().unwrap();
        let bij = bij_for(port, &target);
        bij.download().await;
        assert_eq!(std::fs::read_to_string(&bij.file_path).unwrap(), "good");
    }
}
