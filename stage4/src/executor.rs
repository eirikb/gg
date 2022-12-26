use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;

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

pub trait Executor {
    fn prep(&self, input: AppInput) -> Pin<Box<dyn Future<Output=()>>>;
    fn get_bin(&self, input: AppInput) -> &str;
    fn get_path(&self) -> &str;
    fn before_exec<'a>(&'a self, input: AppInput, command: &'a mut Command) -> Pin<Box<dyn Future<Output=Option<String>> + 'a>>;
}

pub async fn prep(executor: &dyn Executor, input: AppInput) -> Result<AppPath, String> {
    let bin = executor.get_bin(input.clone());
    let path = executor.get_path();
    prep_bin(bin, path, || executor.prep(input.clone())).await
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

async fn prep_bin(bin: &str, path: &str, prep: impl Fn() -> Pin<Box<dyn Future<Output=()>>>) -> Result<AppPath, String> {
    println!("Find {bin} in {path}");
    let app_path = get_app_path(bin, path);

    println!("and path is {:?}", app_path);
    if !(app_path.is_ok() && app_path.unwrap().bin.exists()) {
        println!("prep it!");
        prep().await;
        println!("prep done yo!");
    }
    get_app_path(bin, path)
}

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
