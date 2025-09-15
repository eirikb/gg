use log::info;
use std::path::Path;
use std::process::Command;

pub fn install_gem_to_cache(
    gem_file: &str,
    cache_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Installing gem {} to cache {}", gem_file, cache_path);

    let gem_home = Path::new(cache_path).join("gem_home");
    std::fs::create_dir_all(&gem_home)?;

    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let ruby_gem_bin = Path::new(&home_dir)
        .join(".cache")
        .join("gg")
        .join("ruby")
        .join("ruby_star_")
        .join("bin")
        .join("gem");

    let output = Command::new(&ruby_gem_bin)
        .args(["install", gem_file, "--no-document", "--install-dir"])
        .arg(&gem_home)
        .env("GEM_HOME", &gem_home)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to install gem: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let gem_bin_dir = gem_home.join("bin");
    if gem_bin_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&gem_bin_dir) {
            for entry in entries.flatten() {
                if entry.file_type().unwrap().is_file() {
                    let exe_path = entry.path();
                    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                    let ruby_bin = Path::new(&home_dir)
                        .join(".cache")
                        .join("gg")
                        .join("ruby")
                        .join("ruby_star_")
                        .join("bin")
                        .join("ruby");

                    if let Ok(content) = std::fs::read_to_string(&exe_path) {
                        let lines: Vec<&str> = content.lines().collect();
                        if let Some(first_line) = lines.first() {
                            if first_line.starts_with("#!")
                                && (first_line.contains("ruby")
                                    || first_line.contains("/usr/bin/env"))
                            {
                                let mut new_content = content.replacen(
                                    first_line,
                                    &format!("#!{}", ruby_bin.to_string_lossy()),
                                    1,
                                );

                                let env_setup = "\n# Set gem environment to use gg's cache\nENV['GEM_HOME'] = File.expand_path('../..', __FILE__)\nENV['GEM_PATH'] = File.expand_path('../..', __FILE__)\n";

                                if let Some(require_pos) = new_content.find("require 'rubygems'") {
                                    new_content.insert_str(require_pos, env_setup);
                                } else if let Some(require_pos) = new_content.find("require ") {
                                    new_content.insert_str(require_pos, env_setup);
                                } else {
                                    if let Some(newline_pos) = new_content.find('\n') {
                                        new_content.insert_str(newline_pos + 1, env_setup);
                                    }
                                }

                                let _ = std::fs::write(&exe_path, new_content);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
