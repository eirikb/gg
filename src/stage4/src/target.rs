use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Arch {
    X86_64,
    Armv7,
    Arm64,
    Any,
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Os {
    Windows,
    Linux,
    Mac,
    Any,
}

#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Variant {
    Musl,
    Any,
}

#[derive(Debug, Copy, Clone)]
pub struct Target {
    pub arch: Arch,
    pub os: Os,
    pub variant: Option<Variant>,
}

impl Target {
    pub fn parse_with_overrides(
        input: &str,
        os_override: Option<String>,
        arch_override: Option<String>,
    ) -> Target {
        let parts = input.split("-").collect::<Vec<_>>();

        let arch = if let Some(arch_str) = arch_override {
            match arch_str.to_lowercase().as_str() {
                "x86_64" | "x64" | "amd64" => Arch::X86_64,
                "arm64" | "aarch64" => Arch::Arm64,
                "armv7" | "arm" => Arch::Armv7,
                _ => {
                    eprintln!(
                        "Warning: Unknown architecture '{}', falling back to auto-detection",
                        arch_str
                    );
                    Self::detect_arch_from_input(&parts, input)
                }
            }
        } else {
            Self::detect_arch_from_input(&parts, input)
        };

        let os = if let Some(os_str) = os_override {
            match os_str.to_lowercase().as_str() {
                "windows" | "win" => Os::Windows,
                "linux" => Os::Linux,
                "mac" | "macos" | "darwin" => Os::Mac,
                _ => {
                    eprintln!(
                        "Warning: Unknown OS '{}', falling back to auto-detection",
                        os_str
                    );
                    Self::detect_os_from_input(input)
                }
            }
        } else {
            Self::detect_os_from_input(input)
        };

        Target {
            arch,
            os,
            variant: match input.to_lowercase() {
                x if x.contains("musl") => Some(Variant::Musl),
                _ => None,
            },
        }
    }

    fn detect_arch_from_input(parts: &[&str], _input: &str) -> Arch {
        match parts.get(0).unwrap_or(&"") {
            x if x.contains("x86_64") => Arch::X86_64,
            x if x.contains("arm64") => Arch::Arm64,
            x if x.contains("aarch64") => Arch::Arm64,
            _ => Arch::Armv7,
        }
    }

    fn detect_os_from_input(input: &str) -> Os {
        match input.to_lowercase() {
            x if x.contains("windows") => Os::Windows,
            x if x.contains("apple") => Os::Mac,
            _ => Os::Linux,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_linux_gnu() {
        let target = Target::parse_with_overrides("x86_64-unknown-linux-gnu", None, None);
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn x86_64_windows() {
        let target = Target::parse_with_overrides("x86_64-pc-windows-msvc", None, None);
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Windows, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn x86_64_apple_darwin() {
        let target = Target::parse_with_overrides("x86_64-apple-darwin", None, None);
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn x86_64_unknown_linux_musl() {
        let target = Target::parse_with_overrides("x86_64-unknown-linux-musl", None, None);
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(Some(Variant::Musl), target.variant);
    }

    #[test]
    fn armv7_unknown_linux_gnu() {
        let target = Target::parse_with_overrides("armv7-unknown-linux-gnu", None, None);
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn arm64_apple_darwin() {
        let target = Target::parse_with_overrides("aarch64-apple-darwin", None, None);
        assert_eq!(Arch::Arm64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn os_override_windows() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-gnu",
            Some("windows".to_string()),
            None,
        );
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Windows, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn os_override_mac_variations() {
        let target1 =
            Target::parse_with_overrides("x86_64-pc-windows-msvc", Some("mac".to_string()), None);
        let target2 =
            Target::parse_with_overrides("x86_64-pc-windows-msvc", Some("macos".to_string()), None);
        let target3 = Target::parse_with_overrides(
            "x86_64-pc-windows-msvc",
            Some("darwin".to_string()),
            None,
        );

        assert_eq!(Os::Mac, target1.os);
        assert_eq!(Os::Mac, target2.os);
        assert_eq!(Os::Mac, target3.os);
    }

    #[test]
    fn os_override_linux() {
        let target =
            Target::parse_with_overrides("x86_64-apple-darwin", Some("linux".to_string()), None);
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn arch_override_arm64() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-gnu",
            None,
            Some("arm64".to_string()),
        );
        assert_eq!(Arch::Arm64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn arch_override_x86_64_variations() {
        let target1 = Target::parse_with_overrides(
            "armv7-unknown-linux-gnu",
            None,
            Some("x86_64".to_string()),
        );
        let target2 =
            Target::parse_with_overrides("armv7-unknown-linux-gnu", None, Some("x64".to_string()));
        let target3 = Target::parse_with_overrides(
            "armv7-unknown-linux-gnu",
            None,
            Some("amd64".to_string()),
        );

        assert_eq!(Arch::X86_64, target1.arch);
        assert_eq!(Arch::X86_64, target2.arch);
        assert_eq!(Arch::X86_64, target3.arch);
    }

    #[test]
    fn arch_override_armv7() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-gnu",
            None,
            Some("armv7".to_string()),
        );
        assert_eq!(Arch::Armv7, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn both_overrides() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-gnu",
            Some("windows".to_string()),
            Some("arm64".to_string()),
        );
        assert_eq!(Arch::Arm64, target.arch);
        assert_eq!(Os::Windows, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn overrides_with_musl_variant() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-musl",
            Some("mac".to_string()),
            Some("arm64".to_string()),
        );
        assert_eq!(Arch::Arm64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(Some(Variant::Musl), target.variant);
    }

    #[test]
    fn unknown_os_override_fallback() {
        let target = Target::parse_with_overrides(
            "x86_64-apple-darwin",
            Some("unknown_os".to_string()),
            None,
        );
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Mac, target.os);
        assert_eq!(None, target.variant);
    }

    #[test]
    fn unknown_arch_override_fallback() {
        let target = Target::parse_with_overrides(
            "x86_64-unknown-linux-gnu",
            None,
            Some("unknown_arch".to_string()),
        );
        assert_eq!(Arch::X86_64, target.arch);
        assert_eq!(Os::Linux, target.os);
        assert_eq!(None, target.variant);
    }
}
