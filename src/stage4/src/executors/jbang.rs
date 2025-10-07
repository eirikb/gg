use regex::Regex;
use std::fs;
use std::future::Future;
use std::pin::Pin;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep};
use crate::executors::github::GitHub;

pub struct JBangExecutor {
    github: GitHub,
}

impl JBangExecutor {
    pub fn new(executor_cmd: ExecutorCmd) -> Self {
        let github = GitHub::new_with_config(
            executor_cmd,
            "jbangdev".to_string(),
            "jbang".to_string(),
            None,
            Some(vec![
                "jbang".to_string(),
                "jbang.ps1".to_string(),
                "jbang.cmd".to_string(),
            ]),
        );

        Self { github }
    }
}

impl Executor for JBangExecutor {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        self.github.get_executor_cmd()
    }

    fn get_download_urls<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        self.github.get_download_urls(input)
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        self.github.get_bins(input)
    }

    fn get_name(&self) -> &str {
        self.github.get_name()
    }

    fn get_deps<'a>(
        &'a self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            let java_version = if let Some(file_path) = input.app_args.first() {
                get_jbang_java_version_from_file(file_path).await
            } else {
                None
            };

            vec![ExecutorDep::new("java".to_string(), java_version)]
        })
    }
}

pub async fn get_jbang_java_version_from_file(file_path: &str) -> Option<String> {
    if let Ok(content) = fs::read_to_string(file_path) {
        return parse_jbang_java_version(&content);
    }

    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        // Gotta rewrite URLS to raw.githubusercontent.com to get the content - I believe jbang does the same internally
        let url = if file_path.contains("github.com") && file_path.contains("/blob/") {
            file_path
                .replace("/blob/", "/")
                .replace("github.com", "raw.githubusercontent.com")
        } else {
            file_path.to_string()
        };

        if let Ok(response) = reqwest::get(&url).await {
            if let Ok(content) = response.text().await {
                return parse_jbang_java_version(&content);
            }
        }
    }

    None
}

fn parse_jbang_java_version(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().take(20).collect();

    let java_version_regex = Regex::new(r"^\s*//\s*JAVA\s+(\S+)").ok()?;

    for line in lines {
        if let Some(captures) = java_version_regex.captures(line) {
            if let Some(version) = captures.get(1) {
                let version_str = version.as_str();

                return if version_str.ends_with('+') {
                    let base_version = version_str.trim_end_matches('+');
                    Some(format!(">={}", base_version))
                } else {
                    Some(version_str.to_string())
                };
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jbang_java_version_simple() {
        let content = r#"///usr/bin/env jbang "$0" "$@" ; exit $?
//JAVA 14

public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello World");
    }
}"#;
        assert_eq!(parse_jbang_java_version(content), Some("14".to_string()));
    }

    #[test]
    fn test_parse_jbang_java_version_with_plus() {
        let content = r#"///usr/bin/env jbang "$0" "$@" ; exit $?
//JAVA 21+

public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello World");
    }
}"#;
        assert_eq!(parse_jbang_java_version(content), Some(">=21".to_string()));
    }

    #[test]
    fn test_parse_jbang_java_version_with_spaces() {
        let content = r#"///usr/bin/env jbang "$0" "$@" ; exit $?
// JAVA   17

public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello World");
    }
}"#;
        assert_eq!(parse_jbang_java_version(content), Some("17".to_string()));
    }

    #[test]
    fn test_parse_jbang_java_version_not_found() {
        let content = r#"public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello World");
    }
}"#;
        assert_eq!(parse_jbang_java_version(content), None);
    }

    #[test]
    fn test_parse_jbang_java_version_exact() {
        let content = r#"///usr/bin/env jbang "$0" "$@" ; exit $?
//JAVA 11

public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello World");
    }
}"#;
        assert_eq!(parse_jbang_java_version(content), Some("11".to_string()));
    }
}
