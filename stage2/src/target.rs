#[derive(PartialEq)]
#[derive(strum_macros::Display)]
#[derive(Debug)]
pub enum Arch { X86_64, Armv7 }

#[derive(PartialEq)]
#[derive(strum_macros::Display)]
#[derive(Debug)]
pub enum Os { Windows, Linux, Mac }

pub struct Target {
    pub arch: Arch,
    pub os: Os,
}

pub fn parse_target(input: &str) -> Target {
    let parts = input.split("-").collect::<Vec<_>>();
    return Target {
        arch: match parts[0] {
            x if x.contains("x86_64") => Arch::X86_64,
            _ => Arch::Armv7
        },
        os: match input.to_lowercase() {
            x if x.contains("windows") => Os::Windows,
            x if x.contains("apple") => Os::Mac,
            _ => Os::Linux
        },
    };
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_linux_gnu() {
        let target = parse_target("x86_64-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
    }

    #[test]
    fn x86_64_windows() {
        let target = parse_target("x86_64-pc-windows-msvc");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Windows, target.os);
    }

    #[test]
    fn x86_64_apple_darwin() {
        let target = parse_target("x86_64-apple-darwin");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Mac, target.os);
    }

    #[test]
    fn x86_64_unknown_linux_musl() {
        let target = parse_target("x86_64-unknown-linux-musl");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
    }

    #[test]
    fn armv7_unknown_linux_gnu() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
    }

    #[test]
    fn armv7_unknown_linux_musl() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
    }
}