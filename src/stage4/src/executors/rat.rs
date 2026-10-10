use std::fs;
use std::fs::rename;
use std::future::Future;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::pin::Pin;

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::fetch::fetch_json;
use crate::target::{Arch, Os, Variant};

const BASE_URL: &str = "https://ratbinsa.z1.web.core.windows.net";

// Names look like bin/rat-3.9.0-linux-x64.bin
fn name_to_download(name: &str) -> Download {
    let mut parts = name.split('-');
    let version = parts.nth(1).unwrap_or("NA");
    let os = match parts.next() {
        Some("windows") => Some(Os::Windows),
        Some("linux") => Some(Os::Linux),
        Some("macos") => Some(Os::Mac),
        _ => None,
    };
    // Exact match on the arch part only. An unknown arch stays None, which
    // the download filter drops
    let arch = match parts.next().and_then(|part| part.split('.').next()) {
        Some("x64") => Some(Arch::X86_64),
        Some("arm64") | Some("aarch64") => Some(Arch::Arm64),
        _ => None,
    };
    Download {
        version: GgVersion::new(version),
        tags: Default::default(),
        download_url: format!("{}/{}", BASE_URL, name),
        arch,
        os,
        variant: Some(Variant::Any),
    }
}

pub struct Rat {
    pub executor_cmd: ExecutorCmd,
}

impl Executor for Rat {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move {
            let versions: Vec<String> = match fetch_json(&format!("{}/list.json", BASE_URL)).await {
                Some(versions) => versions,
                None => return vec![],
            };
            versions
                .into_iter()
                .map(|name| name_to_download(&name))
                .collect()
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => "rat.exe",
                _ => "rat.bin",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "rat"
    }

    fn post_prep(&self, cache_path: &str) {
        let patterns = [
            format!("{}/*.bin", cache_path),
            format!("{}/*.exe", cache_path),
        ];

        for pattern in &patterns {
            if let Ok(paths) = glob::glob(pattern) {
                for path in paths.flatten() {
                    if let Some(path_str) = path.to_str() {
                        let to_path = if path_str.ends_with(".bin") {
                            Some(format!("{}/rat.bin", cache_path))
                        } else if path_str.ends_with(".exe") {
                            Some(format!("{}/rat.exe", cache_path))
                        } else {
                            None
                        };
                        if let Some(to_path) = to_path {
                            rename(&path, &to_path).unwrap();
                            #[cfg(unix)]
                            {
                                let mut perms = fs::metadata(&to_path).unwrap().permissions();
                                perms.set_mode(0o755);
                                fs::set_permissions(to_path, perms).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::name_to_download;
    use crate::target::{Arch, Os};

    #[test]
    fn test_name_to_download_x64() {
        let download = name_to_download("bin/rat-3.9.0-linux-x64.bin");
        assert_eq!(download.os, Some(Os::Linux));
        assert_eq!(download.arch, Some(Arch::X86_64));
        assert_eq!(
            download.version.map(|v| v.to_string()),
            Some("3.9.0".to_string())
        );
        assert_eq!(
            download.download_url,
            "https://ratbinsa.z1.web.core.windows.net/bin/rat-3.9.0-linux-x64.bin"
        );
    }

    #[test]
    fn test_name_to_download_x64_other_os() {
        let download = name_to_download("bin/rat-3.9.0-macos-x64.bin");
        assert_eq!(download.os, Some(Os::Mac));
        assert_eq!(download.arch, Some(Arch::X86_64));

        let download = name_to_download("bin/rat-3.9.0-windows-x64.exe");
        assert_eq!(download.os, Some(Os::Windows));
        assert_eq!(download.arch, Some(Arch::X86_64));
    }

    #[test]
    fn test_name_to_download_arm64() {
        let download = name_to_download("bin/rat-4.0.0-macos-arm64.bin");
        assert_eq!(download.os, Some(Os::Mac));
        assert_eq!(download.arch, Some(Arch::Arm64));

        let download = name_to_download("bin/rat-4.0.0-windows-arm64.exe");
        assert_eq!(download.os, Some(Os::Windows));
        assert_eq!(download.arch, Some(Arch::Arm64));
    }

    #[test]
    fn test_name_to_download_unknown_arch() {
        for name in [
            "bin/rat-4.0.0-linux-riscv.bin",
            "bin/rat-4.0.0-linux-x86.bin",
            "bin/rat-4.0.0-linux-notarm64.bin",
            "bin/rat-4.0.0-linux.bin",
        ] {
            assert_eq!(name_to_download(name).arch, None, "{}", name);
        }
    }

    #[test]
    fn test_name_to_download_arch_only_from_its_own_part() {
        let download = name_to_download("arm64/rat-4.0.0-linux-x64.bin");
        assert_eq!(download.arch, Some(Arch::X86_64));
    }
}
