use crate::{Executor, Gradle, Java, Node};

impl dyn Executor {
    pub fn from_cmd(cmd: &str) -> Option<Box<Self>> {
        match cmd {
            "node" | "npm" | "npx" => Some(Box::new(Node { cmd: cmd.to_string() })),
            "gradle" => Some(Box::new(Gradle {})),
            "java" => Some(Box::new(Java {})),
            _ => None
        }
    }
}
