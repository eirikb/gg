use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use crate::download_unpack_and_all_that_stuff;
use crate::target::Target;

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Clone)]
pub struct AppPath {
    pub app: PathBuf,
    pub bin: PathBuf,
}

#[derive(Clone)]
pub struct AppInput {
    pub target: Target,
    pub cmd: String,
}

#[derive(Debug, Clone)]
pub struct Download {
    pub version: String,
    pub lts: bool,
    pub download_url: String,
}

pub trait Executor {
    fn get_download_urls(&self, input: AppInput) -> Pin<Box<dyn Future<Output=Vec<Download>>>>;
    fn get_bin(&self, input: AppInput) -> &str;
    fn get_name(&self) -> &str;
    fn before_exec<'a>(&'a self, _input: AppInput, _command: &'a mut Command) -> Pin<Box<dyn Future<Output=Option<String>> + 'a>> {
        Box::pin(async move { None })
    }
}

pub async fn prep(executor: &dyn Executor, input: AppInput) -> Result<AppPath, String> {
    let bin = executor.get_bin(input.clone());
    let path = executor.get_name();
    println!("Find {bin} in {path}");
    let app_path: Result<AppPath, String> = get_app_path(bin, path);

    match app_path {
        Ok(app_path_ok) if app_path_ok.bin.exists() => return Ok(app_path_ok),
        _ => {}
    }

    println!("prep it!");
    let urls = executor.get_download_urls(input.clone()).await;
    let url_string = urls[0].clone().download_url;
    dbg!(url_string.as_str());
    let cache_path = format!(".cache/{path}");
    download_unpack_and_all_that_stuff(url_string.as_str(), cache_path.as_str()).await;
    println!("prep done yo!");

    get_app_path(bin, path)
}

pub async fn try_execute(executor: &dyn Executor, input: AppInput) -> Result<(), String> {
    let app_path = prep(executor, input.clone()).await?.clone();
    println!("path is {:?}", app_path);
    if app_path.bin.exists() {
        return if try_run(executor, input.clone(), app_path).await.unwrap() {
            Ok(())
        } else {
            Err("Unable to execute".to_string())
        };
    }
    Ok(())
}

fn get_app_path(bin: &str, path: &str) -> Result<AppPath, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache")
        .join(path);

    let dir_entry = path
        .read_dir()
        .map_err(|_| ".cache not found")?
        .next()
        .ok_or("")?;

    let app_path = dir_entry
        .map_err(|_| "app dir not found")?
        .path();


    let bin_path = app_path.join(bin);

    Ok(AppPath { app: app_path, bin: bin_path })
}

// async fn prep_bin(bin: &str, path: &str, prep: impl Fn() -> Pin<Box<dyn Future<Output=()>>>) -> Result<AppPath, String> {
//     println!("Find {bin} in {path}");
//     let app_path = get_app_path(bin, path);
//
//     println!("and path is {:?}", app_path);
//     if !(app_path.is_ok() && app_path.unwrap().bin.exists()) {
//         println!("prep it!");
//         prep().await;
//         println!("prep done yo!");
//     }
//     get_app_path(bin, path)
// }

async fn try_run(executor: &dyn Executor, input: AppInput, app_path: AppPath) -> Result<bool, String> {
    let bin_path = app_path.bin.to_str().unwrap_or("");
    println!("Executing: {:?}", bin_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let path = format!("{bin_path}:{path_string}");
    println!("PATH: {path}");
    let mut command = Command::new(&bin_path);
    let more_path = executor.before_exec(input, &mut command).await;
    let res = command
        .env("PATH", match more_path {
            None => path,
            Some(p) => format!("{p}:{path}")
        })
        .args(env::args().skip(2))
        .spawn().map_err(|_| "What")?.wait().map_err(|_| "eh")?.success();
    if !res {
        println!("Unable to execute {bin_path}");
    }

    Ok(res)
}
