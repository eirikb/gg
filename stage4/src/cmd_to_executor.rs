use crate::{Executor, Gradle, Java, Node};

pub fn cmd_to_executor(cmd: String) -> Option<Box<dyn Executor>> {
    match cmd.as_str() {
        "node" | "npm" | "npx" => Some(Box::new(Node { cmd })),
        "gradle" => Some(Box::new(Gradle {})),
        "java" => Some(Box::new(Java {})),
        _ => None
    }
}

