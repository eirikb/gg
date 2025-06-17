use std::env;
use std::fs;
use std::io::{self, Write};

pub fn clean_cache() -> io::Result<()> {
    let cache_base_dir = env::var("GG_CACHE_DIR").unwrap_or_else(|_| ".cache/gg".to_string());

    if !std::path::Path::new(&cache_base_dir).exists() {
        println!("Cache directory does not exist: {}", cache_base_dir);
        return Ok(());
    }

    println!("Cache directory: {}", cache_base_dir);
    println!("Contents to be deleted:");

    let mut total_size = 0u64;
    let mut file_count = 0u32;
    let mut dir_count = 0u32;

    let pattern = format!("{}/*", cache_base_dir);
    if let Ok(paths) = glob::glob(&pattern) {
        for path in paths.flatten() {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Some(name) = path.file_name() {
                    if metadata.is_dir() {
                        println!("  📁 {}/", name.to_string_lossy());
                    } else {
                        println!("  📄 {}", name.to_string_lossy());
                    }
                }
            }
        }
    }

    let pattern = format!("{}/**/*", cache_base_dir);
    if let Ok(paths) = glob::glob(&pattern) {
        for path in paths.flatten() {
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.is_dir() {
                    dir_count += 1;
                } else {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }

    if file_count == 0 && dir_count <= 1 {
        println!("  (empty)");
    } else {
        println!(
            "\nTotal: {} files in {} directories, {:.1} MB",
            file_count,
            dir_count - 1,
            total_size as f64 / 1024.0 / 1024.0
        );
    }

    print!("\nAre you sure you want to delete the entire cache? (y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            println!("Deleting cache...");
            match fs::remove_dir_all(&cache_base_dir) {
                Ok(()) => println!("Cache cleaned successfully!"),
                Err(e) => println!("Error cleaning cache: {}", e),
            }
        } else {
            println!("Cache cleaning cancelled.");
        }
    } else {
        println!("Cache cleaning cancelled.");
    }

    Ok(())
}
