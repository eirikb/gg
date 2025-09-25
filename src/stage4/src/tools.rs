use std::collections::HashMap;
use std::sync::LazyLock;

use crate::executor::{Executor, ExecutorCmd, ExecutorDep};
use crate::executors::bld::Bld;
use crate::executors::custom_command::CustomCommand;
use crate::executors::flutter::Flutter;
use crate::executors::github::GitHub;
use crate::executors::go::Go;
use crate::executors::gradle::Gradle;
use crate::executors::java::Java;
use crate::executors::jbang::JBangExecutor;
use crate::executors::maven::Maven;
use crate::executors::node::Node;
use crate::executors::openapigenerator::OpenAPIGenerator;
use crate::executors::rat::Rat;
use crate::executors::ruby::Ruby;

#[derive(Clone, Debug)]
pub enum ToolCategory {
    Language,
    BuildTool,
    Utility,
    GitHubRelease,
}

#[derive(Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,
    pub description: &'static str,
    pub category: ToolCategory,
    pub tags: Vec<&'static str>,
    pub example: Option<&'static str>,
    pub factory: fn(ExecutorCmd) -> Option<Box<dyn Executor>>,
}

pub static TOOL_REGISTRY: LazyLock<HashMap<&'static str, ToolInfo>> = LazyLock::new(|| {
    let tools = vec![
        ToolInfo {
            name: "node",
            aliases: vec!["npm", "npx"],
            description: "Node.js JavaScript runtime",
            category: ToolCategory::Language,
            tags: vec!["+lts"],
            example: Some("gg node@14 -e 'console.log(1)'"),
            factory: |cmd| Some(Box::new(Node { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "java",
            aliases: vec![],
            description: "Java runtime and development kit",
            category: ToolCategory::Language,
            tags: vec![
                "+jdk",
                "+jre",
                "+lts",
                "+sts",
                "+mts",
                "+ea",
                "+ga",
                "+headless",
                "+headfull",
                "+fx",
                "+normal",
                "+hotspot",
            ],
            example: Some("gg java@17 -version"),
            factory: |cmd| Some(Box::new(Java { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "go",
            aliases: vec![],
            description: "Go programming language",
            category: ToolCategory::Language,
            tags: vec!["+beta"],
            example: Some("gg go version"),
            factory: |cmd| Some(Box::new(Go { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "flutter",
            aliases: vec!["dart"],
            description: "Flutter SDK for building multi-platform apps",
            category: ToolCategory::Language,
            tags: vec![],
            example: Some("gg flutter --version"),
            factory: |cmd| Some(Box::new(Flutter { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "ruby",
            aliases: vec!["gem", "irb", "bundle"],
            description: "Ruby programming language",
            category: ToolCategory::Language,
            tags: vec![],
            example: Some("gg ruby --version"),
            factory: |cmd| Some(Box::new(Ruby { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "gradle",
            aliases: vec![],
            description: "Gradle build automation tool",
            category: ToolCategory::BuildTool,
            tags: vec![],
            example: Some("gg gradle@6:java@17 clean build"),
            factory: |cmd| Some(Box::new(Gradle::new(cmd))),
        },
        ToolInfo {
            name: "maven",
            aliases: vec!["mvn"],
            description: "Apache Maven build tool",
            category: ToolCategory::BuildTool,
            tags: vec![],
            example: Some("gg maven compile"),
            factory: |cmd| Some(Box::new(Maven { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "bld",
            aliases: vec![],
            description: "Bld build tool",
            category: ToolCategory::BuildTool,
            tags: vec![],
            example: Some("gg bld version"),
            factory: |cmd| Some(Box::new(Bld::new(cmd))),
        },
        ToolInfo {
            name: "jbang",
            aliases: vec![],
            description: "JBang - Java scripting",
            category: ToolCategory::Utility,
            tags: vec![],
            example: Some("gg jbang hello.java"),
            factory: |cmd| Some(Box::new(JBangExecutor::new(cmd))),
        },
        ToolInfo {
            name: "openapi",
            aliases: vec![],
            description: "OpenAPI Generator for API client/server code",
            category: ToolCategory::Utility,
            tags: vec!["+beta"],
            example: Some("gg openapi help"),
            factory: |cmd| Some(Box::new(OpenAPIGenerator { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "rat",
            aliases: vec!["ra"],
            description: "Apache RAT - Release Audit Tool",
            category: ToolCategory::Utility,
            tags: vec![],
            example: Some("gg rat --help"),
            factory: |cmd| Some(Box::new(Rat { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "run",
            aliases: vec![],
            description: "Run any arbitrary command",
            category: ToolCategory::Utility,
            tags: vec![],
            example: Some("gg run:java@17 soapui"),
            factory: |cmd| Some(Box::new(CustomCommand { executor_cmd: cmd })),
        },
        ToolInfo {
            name: "deno",
            aliases: vec![],
            description: "Deno - A secure runtime for JavaScript and TypeScript",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg deno --version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "denoland",
                    "deno",
                    vec![],
                    vec!["deno", "deno.exe"],
                ))
            },
        },
        ToolInfo {
            name: "caddy",
            aliases: vec![],
            description: "Caddy web server",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg caddy version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "caddyserver",
                    "caddy",
                    vec![],
                    vec!["caddy", "caddy.exe"],
                ))
            },
        },
        ToolInfo {
            name: "gh",
            aliases: vec![],
            description: "GitHub CLI",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg gh --version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "cli",
                    "cli",
                    vec![ExecutorDep::optional("git".to_string(), None)],
                    vec!["gh", "gh.exe"],
                ))
            },
        },
        ToolInfo {
            name: "just",
            aliases: vec![],
            description: "Just - A command runner",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg just --version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "casey",
                    "just",
                    vec![],
                    vec!["just", "just.exe"],
                ))
            },
        },
        ToolInfo {
            name: "fortio",
            aliases: vec![],
            description: "Fortio - HTTP/gRPC load testing tool",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg fortio version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "fortio",
                    "fortio",
                    vec![],
                    vec!["bin/fortio", "fortio.exe"],
                ))
            },
        },
        ToolInfo {
            name: "fastlane",
            aliases: vec![],
            description: "Fastlane - iOS and Android automation tool",
            category: ToolCategory::Language,
            tags: vec![],
            example: Some("gg fastlane --version"),
            factory: |cmd| {
                let mut ruby_cmd = cmd.clone();
                ruby_cmd.gems = Some(vec!["fastlane".to_string()]);
                Some(Box::new(Ruby { executor_cmd: ruby_cmd }))
            },
        },
        ToolInfo {
            name: "git",
            aliases: vec![],
            description: "Git version control system",
            category: ToolCategory::GitHubRelease,
            tags: vec![],
            example: Some("gg git --version"),
            factory: |cmd| {
                Some(create_github_executor(
                    cmd,
                    "eirikb",
                    "portable-git",
                    vec![],
                    vec!["git", "git.exe"],
                ))
            },
        },
    ];

    let mut registry = HashMap::new();
    for tool in tools {
        registry.insert(tool.name, tool.clone());
        for alias in &tool.aliases {
            registry.insert(alias, tool.clone());
        }
    }
    registry
});

fn create_github_executor(
    executor_cmd: ExecutorCmd,
    owner: &str,
    repo: &str,
    deps: Vec<ExecutorDep>,
    bins: Vec<&str>,
) -> Box<dyn Executor> {
    Box::new(GitHub::new_with_config(
        executor_cmd,
        owner.to_string(),
        repo.to_string(),
        Some(deps),
        Some(bins.into_iter().map(|s| s.to_string()).collect()),
    ))
}

pub fn get_tool_info(name: &str) -> Option<&'static ToolInfo> {
    TOOL_REGISTRY.get(name)
}

pub fn get_all_tools() -> Vec<&'static ToolInfo> {
    let mut tools: Vec<_> = TOOL_REGISTRY
        .values()
        .filter(|tool| !tool.aliases.contains(&tool.name))
        .collect();
    tools.sort_by(|a, b| a.name.cmp(b.name));
    tools.dedup_by(|a, b| a.name == b.name);
    tools
}
