#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
pub enum Arch { X86_64, Armv7, Arm64 }

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
pub enum Os { Windows, Linux, Mac }

#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
pub enum Variant { Musl, Gnu, Msvc }

#[derive(Debug)]
#[derive(Copy, Clone)]
pub struct Target {
    pub arch: Arch,
    pub os: Os,
    pub variant: Option<Variant>,
}

pub fn parse_target(input: &str) -> Target {
    let parts = input.split("-").collect::<Vec<_>>();
    return Target {
        arch: match parts[0] {
            x if x.contains("x86_64") => Arch::X86_64,
            x if x.contains("arm64") => Arch::Arm64,
            _ => Arch::Armv7
        },
        os: match input.to_lowercase() {
            x if x.contains("windows") => Os::Windows,
            x if x.contains("apple") => Os::Mac,
            _ => Os::Linux
        },
        variant: match input.to_lowercase() {
            x if x.contains("musl") => Some(Variant::Musl),
            x if x.contains("gnu") => Some(Variant::Gnu),
            x if x.contains("msvc") => Some(Variant::Msvc),
            _ => None
        },
    };
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_linux_gnu() {
        let target = parse_target("x86_64-unknown-linux-gnu");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Variant::Gnu, target.variant);
    }

    #[test]
    fn x86_64_windows() {
        let target = parse_target("x86_64-pc-windows-msvc");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Windows, target.os);
        assert_eq!(Variant::Msvc, target.variant);
    }

    #[test]
    fn x86_64_apple_darwin() {
        let target = parse_target("x86_64-apple-darwin");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(Variant::None, target.variant);
    }

    #[test]
    fn x86_64_unknown_linux_musl() {
        let target = parse_target("x86_64-unknown-linux-musl");
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Variant::Musl, target.variant);
    }

    #[test]
    fn armv7_unknown_linux_gnu() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Variant::Gnu, target.variant);
    }

    #[test]
    fn armv7_unknown_linux_musl() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Variant::Gnu, target.variant);
    }
}
