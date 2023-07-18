use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub enum Arch { X86_64, Armv7, Arm64, Any }

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub enum Os { Windows, Linux, Mac, Any }

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
#[derive(Serialize, Deserialize)]
pub enum Variant { Musl, Any }

#[derive(Debug)]
#[derive(Copy, Clone)]
pub struct Target {
    pub arch: Arch,
    pub os: Os,
    pub variant: Option<Variant>,
}

impl Target {
    pub fn parse(input: &str) -> Target {
        let parts = input.split("-").collect::<Vec<_>>();
        return Target {
            arch: match parts[0] {
                x if x.contains("x86_64") => Arch::X86_64,
                x if x.contains("arm64") => Arch::Arm64,
                x if x.contains("aarch64") => Arch::Arm64,
                _ => Arch::Armv7
            },
            os: match input.to_lowercase() {
                x if x.contains("windows") => Os::Windows,
                x if x.contains("apple") => Os::Mac,
                _ => Os::Linux
            },
            variant: match input.to_lowercase() {
                x if x.contains("musl") => Some(Variant::Musl),
                _ => None
            },
        };
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_linux_gnu() {
        let target = Target::parse("x86_64-unknown-linux-gnu");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
    }

    #[test]
    fn x86_64_windows() {
        let target = Target::parse("x86_64-pc-windows-msvc");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Windows, target.os);
    }

    #[test]
    fn x86_64_apple_darwin() {
        let target = Target::parse("x86_64-apple-darwin");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn x86_64_unknown_linux_musl() {
        let target = Target::parse("x86_64-unknown-linux-musl");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Some(Variant::Musl), target.variant);
    }

    #[test]
    fn armv7_unknown_linux_gnu() {
        let target = Target::parse("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
    }

    #[test]
    fn armv7_unknown_linux_musl() {
        let target = Target::parse("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
    }
}
