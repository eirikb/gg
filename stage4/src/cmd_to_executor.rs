use std::collections::HashMap;
use semver::VersionReq;
use crate::{Executor, Gradle, Java, Node};


fn get_version_req(cmd: &str, version_req_map: HashMap<String, Option<VersionReq>>) -> Option<VersionReq> {
    if let Some(v) = version_req_map.get(cmd) {
        v.clone()
    } else {
        None
    }
}

pub fn cmd_to_executor(cmd: String) -> Option<Box<dyn Executor>> {
    match cmd.as_str() {
        "node" | "npm" | "npx" => Some(Box::new(Node { cmd })),
        "gradle" => Some(Box::new(Gradle {})),
        "java" => Some(Box::new(Java {})),
        _ => None
    }
}

