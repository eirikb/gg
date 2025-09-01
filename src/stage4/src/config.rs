use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgConfig {
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

impl Default for GgConfig {
    fn default() -> Self {
        Self {
            dependencies: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
}

impl GgConfig {
    pub fn load() -> Self {
        match Self::find_config_file() {
            Some(path) => {
                info!("Loading config from: {}", path.display());
                match Self::load_from_file(&path) {
                    Ok(config) => config,
                    Err(e) => {
                        warn!("Failed to load config from {}: {}", path.display(), e);
                        Self::default()
                    }
                }
            }
            None => {
                debug!("No config file found, using defaults");
                Self::default()
            }
        }
    }

    fn find_config_file() -> Option<PathBuf> {
        let mut current_dir = env::current_dir().ok()?;

        loop {
            let config_path = current_dir.join("gg.toml");
            if config_path.exists() {
                return Some(config_path);
            }

            if !current_dir.pop() {
                break;
            }
        }

        None
    }

    fn load_from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: GgConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn resolve_alias(&self, command: &str) -> Option<Vec<String>> {
        self.aliases
            .get(command)
            .map(|alias_cmd| shlex::split(alias_cmd).unwrap_or_else(|| vec![alias_cmd.clone()]))
    }

    pub fn resolve_alias_with_and(&self, command: &str) -> Option<Vec<Vec<String>>> {
        self.aliases.get(command).map(|alias_cmd| {
            alias_cmd
                .split("&&")
                .map(|cmd| cmd.trim())
                .map(|cmd| shlex::split(cmd).unwrap_or_else(|| vec![cmd.to_string()]))
                .collect()
        })
    }

    pub fn init_config() -> Result<(), String> {
        let config_path = Path::new("gg.toml");

        if config_path.exists() {
            return Err("gg.toml already exists in current directory".to_string());
        }

        let default_config = r#"# gg configuration file
# See https://github.com/eirikb/gg for more information

[dependencies]
# Define version requirements for tools
# Examples:
# node = "^18.0.0"
# java = "17"
# gradle = "~7.6.0"

[aliases]
# Define command shortcuts
# Examples:
# build = "gradle clean build"
# serve = "node@18 server.js"
# test = "npm test"
"#;

        fs::write(config_path, default_config)
            .map_err(|e| format!("Failed to create gg.toml: {}", e))?;

        println!("Created gg.toml configuration file");
        Ok(())
    }

    pub fn show_config(&self) -> Result<(), String> {
        match Self::find_config_file() {
            Some(path) => {
                println!("Configuration loaded from: {}", path.display());
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        println!("\nCurrent configuration:");
                        println!("{}", content);
                    }
                    Err(e) => return Err(format!("Failed to read config file: {}", e)),
                }

                if !self.dependencies.is_empty() || !self.aliases.is_empty() {
                    println!("\nParsed configuration:");
                    if !self.dependencies.is_empty() {
                        println!("\nDependencies:");
                        for (tool, version) in &self.dependencies {
                            println!("  {} = \"{}\"", tool, version);
                        }
                    }

                    if !self.aliases.is_empty() {
                        println!("\nAliases:");
                        for (alias, command) in &self.aliases {
                            println!("  {} = \"{}\"", alias, command);
                        }
                    }
                }
            }
            None => {
                println!("No gg.toml configuration file found");
                println!("Run 'gg config init' to create one");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("gg.toml");

        let config_content = r#"
[dependencies]
node = "^18.0.0"
java = "17"

[aliases]
build = "gradle clean build"
test = "npm test"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = GgConfig::load_from_file(&config_path).unwrap();

        assert_eq!(
            config.dependencies.get("node"),
            Some(&"^18.0.0".to_string())
        );
        assert_eq!(config.dependencies.get("java"), Some(&"17".to_string()));
        assert_eq!(
            config.aliases.get("build"),
            Some(&"gradle clean build".to_string())
        );
        assert_eq!(config.aliases.get("test"), Some(&"npm test".to_string()));
    }

    #[test]
    fn test_alias_resolution() {
        let mut config = GgConfig::default();
        config
            .aliases
            .insert("build".to_string(), "gradle clean build".to_string());
        config
            .aliases
            .insert("quoted".to_string(), r#"echo "hello world""#.to_string());

        let resolved = config.resolve_alias("build").unwrap();
        assert_eq!(resolved, vec!["gradle", "clean", "build"]);

        let quoted_resolved = config.resolve_alias("quoted").unwrap();
        assert_eq!(quoted_resolved, vec!["echo", "hello world"]);

        assert!(config.resolve_alias("nonexistent").is_none());
    }

    #[test]
    fn test_alias_with_and_operator() {
        let mut config = GgConfig::default();
        config.aliases.insert(
            "build-and-test".to_string(),
            "gradle clean build && npm test".to_string(),
        );

        let resolved = config.resolve_alias_with_and("build-and-test").unwrap();
        assert_eq!(
            resolved,
            vec![vec!["gradle", "clean", "build"], vec!["npm", "test"]]
        );
    }
}
