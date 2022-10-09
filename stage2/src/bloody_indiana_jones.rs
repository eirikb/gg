use std::borrow::Borrow;
use std::fs::{create_dir, File};
use std::io;
use std::io::{BufReader, Write};
use std::path::Path;
// use async_compression::futures;
use crate::Url;
// use futures_io::{AsyncRead, AsyncWrite, AsyncBufRead};
use async_compression::tokio::write::XzDecoder;
// use async_compression::futures::bufread::GzipDecoder;
// use tokio::io::AsyncReadExt;
// use futures::stream::TryStreamExt;
// use tokio::io::AsyncBufReadExt;
// use tokio_util::compat::FuturesAsyncReadCompatExt;

pub async fn download_unpack_and_all_that_stuff(url: &String) {
    println!("Downloading {url}");
    let res = reqwest::get(url).await
        .expect("Unable to download");
    create_dir(".cache/gg/downloads");

    let file_name = Url::parse(url).unwrap().path_segments().unwrap().last().unwrap().to_string();

    let file_path = &format!(".cache/gg/downloads/{file_name}");
    let mut out = File::create(file_path)
        .expect("Unable to create archive file");
    let bytes = res.bytes().await.expect("duh");
    // //
    io::copy(&mut bytes.as_ref(), &mut out)
        .expect("Unable to download the file?!");
    println!("Done...");

    // let response = reqwest::get(url).await
    //     .expect("oh noes");
    // let stream = response
    //     .bytes_stream()
    //     .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, "Error!"))
    //     // .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
    //     // .map(|result| result.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, "Error!")))
    //     .into_async_read();
    //     // .compat();
    // let gzip_decoder = GzipDecoder::new(stream);

    // Print decompressed txt content
    // let buf_reader = tokio::io::BufReader::new(gzip_decoder);
    // let mut lines = buf_reader.lines();
    // while let Some(line) = lines.next_line().await? {
    //     println!("{line}");
    // }

    // Ok(())

    // let mut tmpfile = tempfile::tempfile().unwrap();
    // reqwest::get(url).unwrap().copy_to(&mut tmpfile);
    // let mut zip = zip::ZipArchive::new(tmpfile).unwrap();
    // println!("{:#?}", zip);

    // use futures::{
    //     io::{self, BufReader, ErrorKind},
    //     prelude::*,
    // };
    // let response = reqwest::get("http://localhost:8000/test.txt.gz").await?;
    // let stream = res.bytes_stream();
    // let mut stream_reader = StreamReader::new(stream);
    // let mut decoder = GzipDecoder::new(stream);
    // let reader = res.bytes_stream().into_async_read();
    // let mut decoder = GzipDecoder::new(BufReader::new(reader));
    // let mut data = String::new();
    // decoder.read_to_string(&mut data).await?;
    // println!("{data:?}");
    // Ok(())


    //
    println!("Extracting {file_name}");
    // let mut f = io::BufReader::new(File::open(file_path).unwrap());
    // let decoder = XzDecoder::new(BufReader::new(f));
    // io::copy(&mut decoder, &mut f2);
    // let mut f2 = io::BufWriter::new(File::open(file_path + ".tar").unwrap());

    // let mut decomp: Vec<u8> = Vec::new();
    // lzma_rs::xz_decompress(&mut f, &mut decomp).unwrap();
    // io::copy(&mut f, &mut decomp)
    //     .expect("Unable to download the file?!");
    //
    // let file_path_decomp = Path::new(&format!(".cache/gg/downloads/{file_name}")).with_extension("").to_str().unwrap().to_string();
    // println!("Write to {file_path_decomp}");
    // // let mut archive = tar::Archive::
}

#[cfg(test)]
mod test {
    use crate::bloody_indiana_jones::download_unpack_and_all_that_stuff;

    #[tokio::test]
    async fn ok() {
        let url = String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz");
        download_unpack_and_all_that_stuff(&url).await;
    }
}