use crate::target::{Arch, Os};

pub fn create_github_client() -> Result<octocrab::Octocrab, octocrab::Error> {
    octocrab::Octocrab::builder()
        .base_uri("https://ghapi.ggcmd.io/")?
        .build()
}

pub fn detect_os_from_name(name: &str) -> Option<Os> {
    let name_lower = name.to_lowercase();
    if name_lower.contains("darwin") || name_lower.contains("macos") || name_lower.contains("apple")
    {
        Some(Os::Mac)
    } else if name_lower.contains("windows")
        || name_lower.contains("win")
        || name_lower.contains(".exe")
    {
        Some(Os::Windows)
    } else if name_lower.contains("linux") {
        Some(Os::Linux)
    } else {
        None
    }
}

pub fn detect_arch_from_name(name: &str) -> Option<Arch> {
    let name_lower = name.to_lowercase();
    if name_lower.contains("x86_64") || name_lower.contains("amd64") || name_lower.contains("x64") {
        Some(Arch::X86_64)
    } else if name_lower.contains("arm64") || name_lower.contains("aarch64") {
        Some(Arch::Arm64)
    } else if name_lower.contains("armv7") || name_lower.contains("arm") {
        Some(Arch::Armv7)
    } else if name_lower.contains("x86") {
        Some(Arch::Any)
    } else {
        None
    }
}
