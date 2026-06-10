use std::future::Future;
use std::pin::Pin;

use log::{debug, info};

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os, Variant};

const DOWNLOAD_BASE_URL: &str = "https://downloads.claude.ai/claude-code-releases";

/// Claude Code - distributed via Anthropic's native installer endpoints
/// (npm is deprecated). Version resolution: `<base>/latest` returns the
/// latest version string; the binary lives at `<base>/<version>/<platform>/claude`.
pub struct Claude {
    pub executor_cmd: ExecutorCmd,
}

fn platform_string(target: &crate::target::Target) -> Option<String> {
    let arch = match target.arch {
        Arch::X86_64 => "x64",
        Arch::Arm64 => "arm64",
        _ => return None,
    };
    Some(match target.os {
        Os::Linux => match target.variant {
            Some(Variant::Musl) => format!("linux-{arch}-musl"),
            _ => format!("linux-{arch}"),
        },
        Os::Mac => format!("darwin-{arch}"),
        Os::Windows => {
            if arch != "x64" {
                return None;
            }
            "win32-x64".to_string()
        }
        _ => return None,
    })
}

async fn get_claude_downloads(target: &crate::target::Target) -> Vec<Download> {
    let Some(platform) = platform_string(target) else {
        debug!("claude: unsupported platform");
        return vec![];
    };

    let version = match reqwest::get(format!("{DOWNLOAD_BASE_URL}/latest")).await {
        Ok(res) => match res.text().await {
            Ok(text) => text.trim().to_string(),
            Err(e) => {
                info!("claude: failed to read latest version: {e}");
                return vec![];
            }
        },
        Err(e) => {
            info!("claude: failed to fetch latest version: {e}");
            return vec![];
        }
    };

    if !version
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        info!("claude: unexpected version response: {version}");
        return vec![];
    }

    let binary = if target.os == Os::Windows {
        "claude.exe"
    } else {
        "claude"
    };

    vec![Download {
        download_url: format!("{DOWNLOAD_BASE_URL}/{version}/{platform}/{binary}"),
        version: GgVersion::new(&version),
        os: Some(Os::Any),
        arch: Some(Arch::Any),
        variant: Some(Variant::Any),
        tags: Default::default(),
    }]
}

impl Executor for Claude {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        Box::pin(async move { get_claude_downloads(&input.target).await })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => "claude.exe",
                _ => "claude",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "claude"
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        vec![".".to_string()]
    }

    fn post_prep(&self, cache_path: &str) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let bin = std::path::Path::new(cache_path).join("claude");
            if bin.exists() {
                let perms = std::fs::Permissions::from_mode(0o755);
                if let Err(e) = std::fs::set_permissions(&bin, perms) {
                    println!("claude: failed to set executable permission: {e}");
                }
            }
        }
    }
}
