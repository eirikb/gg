use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

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

pub async fn prep_bin(bin: &str, path: &str, prep: impl Fn() -> Pin<Box<dyn Future<Output=()>>>) -> Result<PathBuf, String> {
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

pub fn try_run(bin_path: &str) -> Result<bool, String> {
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
