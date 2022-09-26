pub mod target {
    #[derive(PartialEq)]
    pub enum Arch { X86_64, Armv7 }

    #[derive(PartialEq)]
    pub enum Os { Windows, Linux, Mac }

    pub struct Target {
        pub arch: Arch,
        pub os: Os,
    }

    pub fn parse_target(input: &str) -> Target {
        let parts = input.split("-").collect::<Vec<_>>();
        return Target {
            arch: match parts[0] {
                "x86_64" => Arch::X86_64,
                _ => Arch::Armv7
            },
            os: match input.to_lowercase() {
                x if x.contains("windows") => Os::Windows,
                x if x.contains("apple") => Os::Mac,
                _ => Os::Linux
            },
        };
    }
}


#[cfg(test)]
mod tests {
    use crate::target::target::Arch::{Armv7, X86_64};
    use crate::target::target::Os::{Linux, Mac, Windows};
    use crate::target::target::parse_target;

    #[test]
    fn x86_64_linux_gnu() {
        let target = parse_target("x86_64-unknown-linux-gnu");
        assert_eq!(X86_64, target.arch);
        assert_eq!(Linux, target.os);
    }

    #[test]
    fn x86_64_windows() {
        let target = parse_target("x86_64-pc-windows-msvc");
        assert_eq!(X86_64, target.arch);
        assert_eq!(Windows, target.os);
    }

    #[test]
    fn x86_64_apple_darwin() {
        let target = parse_target("x86_64-apple-darwin");
        assert_eq!(X86_64, target.arch);
        assert_eq!(Mac, target.os);
    }

    #[test]
    fn x86_64_unknown_linux_musl() {
        let target = parse_target("x86_64-unknown-linux-musl");
        assert_eq!(X86_64, target.arch);
        assert_eq!(Linux, target.os);
    }

    #[test]
    fn armv7_unknown_linux_gnu() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Armv7, target.arch);
        assert_eq!(Linux, target.os);
    }

    #[test]
    fn armv7_unknown_linux_musl() {
        let target = parse_target("armv7-unknown-linux-gnu");
        assert_eq!(Armv7, target.arch);
        assert_eq!(Linux, target.os);
    }
}
