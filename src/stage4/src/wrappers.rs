use std::{os::unix::fs::PermissionsExt, path::Path};

use crate::config::GgConfig;
use futures::future::try_join_all;
use git2::Repository;
use tokio::fs;

pub async fn create_wrappers(config: &GgConfig) -> Result<Vec<()>, Box<dyn std::error::Error>> {
    let aliases = config.aliases.keys().chain(config.dependencies.keys());
    let wrappers_dir = Path::new(".gg/wrappers");
    fs::create_dir_all(wrappers_dir).await?;
    let futures = aliases.map(|alias| async move {
        let (sh, bat) = tokio::join!(
            create_sh_script(wrappers_dir, alias),
            create_bat_script(wrappers_dir, alias),
        );
        sh?;
        bat?;
        Ok(())
    });

    try_join_all(futures).await
}

async fn create_sh_script(
    wrappers_dir: &Path,
    alias: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let script_path = wrappers_dir.join(alias);
    fs::write(&script_path, format!("./gg.cmd {alias} \"$@\"")).await?;

    let repo = Repository::open(".")?;
    let mut index = repo.index()?;
    index.add_path(&script_path)?;

    if cfg!(windows) {
        let mut entry = index
            .get_path(&script_path, 0)
            .ok_or("File not found in index")?;
        entry.mode = git2::FileMode::BlobExecutable.into();
        index.add(&entry)?;
    } else {
        let mut permissions = fs::metadata(&script_path).await?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).await?;
    }

    index.write()?;
    Ok(())
}

async fn create_bat_script(wrappers_dir: &Path, alias: &str) -> Result<(), std::io::Error> {
    fs::write(
        wrappers_dir.join(format!("{alias}.bat")),
        format!("@echo off\n\"gg.cmd\" {alias} %*"),
    )
    .await
}
