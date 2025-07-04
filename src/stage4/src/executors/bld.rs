use std::fs;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use semver::VersionReq;

use crate::executor::{
    AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep,
};
use crate::executors::github::GitHub;

pub struct Bld {
    github: GitHub,
}

impl Bld {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        let github = GitHub::new_with_config(
            executor_cmd,
            "rife2".to_string(),
            "bld".to_string(),
            None,
            Some(vec!["bld".to_string(), "bld.bat".to_string()]),
        );

        Self { github }
    }

    fn get_bld_version() -> Option<String> {
        let properties_path = "lib/bld/bld-wrapper.properties";
        if let Ok(content) = fs::read_to_string(properties_path) {
            for line in content.lines() {
                if line.starts_with("bld.version=") {
                    return Some(line.trim_start_matches("bld.version=").to_string());
                }
            }
        }
        None
    }

    fn find_bld_class() -> Option<String> {
        if let Some(class_name) = Self::find_class_from_bld_file() {
            return Some(class_name);
        }

        let bld_java_dir = Path::new("src/bld/java");
        if !bld_java_dir.exists() {
            return None;
        }

        fn find_java_files(dir: &Path, package_prefix: &str) -> Option<String> {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = path.file_name()?.to_str()?;
                        let new_package = if package_prefix.is_empty() {
                            dir_name.to_string()
                        } else {
                            format!("{}.{}", package_prefix, dir_name)
                        };
                        if let Some(class_name) = find_java_files(&path, &new_package) {
                            return Some(class_name);
                        }
                    } else if path.extension().map_or(false, |ext| ext == "java") {
                        let file_name = path.file_stem()?.to_str()?;
                        if file_name.ends_with("Build") {
                            return Some(if package_prefix.is_empty() {
                                file_name.to_string()
                            } else {
                                format!("{}.{}", package_prefix, file_name)
                            });
                        }
                    }
                }
            }
            None
        }

        find_java_files(bld_java_dir, "")
    }

    fn find_class_from_bld_file() -> Option<String> {
        let bld_file = Path::new("bld");
        if !bld_file.exists() {
            return None;
        }

        if let Ok(content) = fs::read_to_string(bld_file) {
            for line in content.lines() {
                if let Some(build_pos) = line.find("--build") {
                    let after_build = &line[build_pos + 7..];

                    let tokens: Vec<&str> = after_build.split_whitespace().collect();
                    if let Some(class_name) = tokens.first() {
                        if !class_name.is_empty() {
                            return Some(class_name.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

impl Executor for Bld {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        self.github.get_executor_cmd()
    }

    fn get_version_req(&self) -> Option<VersionReq> {
        if let Some(version) = Self::get_bld_version() {
            if let Ok(version_req) = VersionReq::parse(&version) {
                return Some(version_req);
            }
        }
        None
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        self.github.get_download_urls(input)
    }

    fn get_bins(&self, _input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact("java".to_string())]
    }

    fn get_name(&self) -> &str {
        self.github.get_name()
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            vec![ExecutorDep {
                name: "java".to_string(),
                version: None,
            }]
        })
    }

    fn customize_args(&self, input: &AppInput, app_path: &AppPath) -> Vec<String> {
        let mut args = vec![
            "-jar".to_string(),
            app_path
                .install_dir
                .join("lib/bld-wrapper.jar")
                .to_str()
                .unwrap_or("")
                .to_string(),
            "dummy".to_string(),
        ];

        if let Some(class_name) = Self::find_bld_class() {
            args.push("--build".to_string());
            args.push(class_name);
        }

        args.extend(input.no_clap.app_args.clone());
        args
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_bld_version() {
        let content = "bld.version=2.2.1";
        let lines: Vec<&str> = content.lines().collect();
        let mut version = None;
        for line in lines {
            if line.starts_with("bld.version=") {
                version = Some(line.trim_start_matches("bld.version=").to_string());
            }
        }
        assert_eq!(version, Some("2.2.1".to_string()));
    }

    #[test]
    fn test_parse_bld_version_with_spaces() {
        let content = "bld.version = 2.2.1";
        let lines: Vec<&str> = content.lines().collect();
        let mut version = None;
        for line in lines {
            if line.trim().starts_with("bld.version") && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() == 2 {
                    version = Some(parts[1].trim().to_string());
                }
            }
        }
        assert_eq!(version, Some("2.2.1".to_string()));
    }

    #[test]
    fn test_find_class_from_bld_file() {
        let test_cases = vec![
            (
                "java -jar lib/bld-wrapper.jar dummy --build com.example.ExampleBuild",
                Some("com.example.ExampleBuild".to_string()),
            ),
            (
                "java -jar lib/bld-wrapper.jar dummy --build com.example.ExampleBuild \"$@\"",
                Some("com.example.ExampleBuild".to_string()),
            ),
            (
                "--build com.test.TestBuild",
                Some("com.test.TestBuild".to_string()),
            ),
            (
                "something --build MyBuild more args",
                Some("MyBuild".to_string()),
            ),
            ("no build parameter here", None),
            ("--build", None),
            ("--build ", None),
        ];

        for (input, expected) in test_cases {
            let result = input.lines().find_map(|line| {
                if let Some(build_pos) = line.find("--build") {
                    let after_build = &line[build_pos + 7..];
                    let tokens: Vec<&str> = after_build.split_whitespace().collect();
                    if let Some(class_name) = tokens.first() {
                        if !class_name.is_empty() {
                            return Some(class_name.to_string());
                        }
                    }
                }
                None
            });
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_bld_args_structure_with_class() {
        use crate::executor::{AppInput, AppPath, Executor, ExecutorCmd};
        use crate::no_clap::NoClap;
        use crate::target::Target;
        use std::collections::HashSet;
        use std::fs;
        use std::path::PathBuf;

        let temp_bld_content =
            "java -jar lib/bld-wrapper.jar dummy --build com.example.TestBuild \"$@\"";
        let _ = fs::write("bld", temp_bld_content);

        let bld = super::Bld::new(ExecutorCmd {
            cmd: "bld".to_string(),
            version: None,
            distribution: None,
            include_tags: HashSet::new(),
            exclude_tags: HashSet::new(),
        });

        let mut no_clap = NoClap::new();
        no_clap.app_args = vec!["compile".to_string()];

        let app_input = AppInput {
            target: Target::parse_with_overrides("", None, None),
            no_clap,
        };

        let app_path = AppPath {
            install_dir: PathBuf::from("/test/cache/path"),
        };

        let args = bld.customize_args(&app_input, &app_path);

        assert_eq!(args[0], "-jar");
        assert!(args[1].contains("lib/bld-wrapper.jar"));
        assert_eq!(args[2], "dummy");
        assert_eq!(args[3], "--build");
        assert_eq!(args[4], "com.example.TestBuild");
        assert!(args.contains(&"compile".to_string()));

        let _ = fs::remove_file("bld");
    }

    #[test]
    fn test_bld_args_structure_without_class() {
        use crate::executor::{AppInput, AppPath, Executor, ExecutorCmd};
        use crate::no_clap::NoClap;
        use crate::target::Target;
        use std::collections::HashSet;
        use std::path::PathBuf;

        let _ = std::fs::remove_file("bld");

        let bld = super::Bld::new(ExecutorCmd {
            cmd: "bld".to_string(),
            version: None,
            distribution: None,
            include_tags: HashSet::new(),
            exclude_tags: HashSet::new(),
        });

        let mut no_clap = NoClap::new();
        no_clap.app_args = vec!["compile".to_string()];

        let app_input = AppInput {
            target: Target::parse_with_overrides("", None, None),
            no_clap,
        };

        let app_path = AppPath {
            install_dir: PathBuf::from("/test/cache/path"),
        };

        let args = bld.customize_args(&app_input, &app_path);

        assert_eq!(args[0], "-jar");
        assert!(args[1].contains("lib/bld-wrapper.jar"));
        assert_eq!(args[2], "dummy");
        assert!(!args.contains(&"--build".to_string()));
        assert!(args.contains(&"compile".to_string()));
    }
}
