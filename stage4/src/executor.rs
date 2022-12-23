use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::target::Target;

pub trait Executor {
    fn prep(&self, target: Target) -> Pin<Box<dyn Future<Output=()>>>;
    fn get_bin(&self, target: Target, v: String) -> &str;
    fn get_path(&self) -> &str;
}

async fn prep(executor: Box<dyn Executor>, target: Target, v: String) -> Result<PathBuf, String> {
    let bin = executor.get_bin(target, v);
    let path = executor.get_path();
    prep_bin(bin, path, || executor.prep(target)).await
}

pub async fn try_execute(executor: Box<dyn Executor>, target: Target, v: String) -> Result<(), String> {
    let bin_path = prep(executor, target, v).await?.clone();
    println!("path is {:?}", bin_path);
    if bin_path.exists() {
        return if try_run(bin_path.to_str().unwrap_or("")).unwrap() {
            Ok(())
        } else {
            Err("Unable to execute".to_string())
        };
    }
    Ok(())
}

fn get_bin_path(bin: &str, path: &str) -> Result<PathBuf, String> {
    let path = env::current_dir()
        .map_err(|_| "Current dir not found")?
        .join(".cache")
        .join(path);

    let dir_entry = path
        .read_dir()
        .map_err(|_| ".cache not found")?
        .next()
        .ok_or("")?;

    let path = dir_entry
        .map_err(|_| "app dir not found")?
        .path()
        .join(bin);

    Ok(path)
}

async fn prep_bin(bin: &str, path: &str, prep: impl Fn() -> Pin<Box<dyn Future<Output=()>>>) -> Result<PathBuf, String> {
    println!("Find {bin} in {path}");
    let bin_path = get_bin_path(bin, path);

    println!("and path is {:?}", bin_path);
    if !(bin_path.is_ok() && bin_path.unwrap().exists()) {
        println!("prep it!");
        prep().await;
        println!("prep done yo!");
    }
    get_bin_path(bin, path)
}

fn try_run(bin_path: &str) -> Result<bool, String> {
    println!("Executing: {:?}", bin_path);
    let path_string = &env::var("PATH").unwrap_or("".to_string());
    let path = format!("{bin_path}/bin:{path_string}");
    println!("PATH: {path}");
    let res = std::process::Command::new(&bin_path)
        .env("PATH", path)
        .args(env::args().skip(2))
        .spawn().map_err(|_| "What")?.wait().map_err(|_| "eh")?.success();
    if !res {
        println!("Unable to execute {bin_path}");
    }

    Ok(res)
}
