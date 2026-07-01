use std::future::Future;
use std::pin::Pin;

use log::{debug, info, warn};

use crate::executor::{AppInput, BinPattern, Download, Executor, ExecutorCmd, GgVersion};
use crate::target::{Arch, Os, Variant};

// Grok Build (xAI's coding agent) ships as a self-contained native binary, not
// an archive. This is the same delivery the official `x.ai/cli/install.sh`
// uses, so we skip the @xai-official/grok npm wrapper (and the Node.js runtime
// it would drag in). Version resolution: `<base>/<channel>` returns the latest
// version string; the binary lives at `<base>/grok-<version>-<platform>[.exe]`.
const BASE_URL_PRIMARY: &str = "https://x.ai/cli";
const BASE_URL_FALLBACK: &str = "https://storage.googleapis.com/grok-build-public-artifacts/cli";
const CHANNEL: &str = "stable";

pub struct Grok {
    pub executor_cmd: ExecutorCmd,
}

/// `<os>-<arch>`, matching install.sh. Only x86_64/aarch64 are published. The
/// linux binary is static-pie linked (no interpreter, no libc NEEDED), so it
/// runs on both glibc and musl/Alpine — hence we don't gate on variant.
fn platform_string(target: &crate::target::Target) -> Option<String> {
    let arch = match target.arch {
        Arch::X86_64 => "x86_64",
        Arch::Arm64 => "aarch64",
        _ => return None,
    };
    let os = match target.os {
        Os::Linux => "linux",
        Os::Mac => "macos",
        Os::Windows => "windows",
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

/// Fetch the latest version from a host's channel file, e.g. `0.2.77`.
async fn fetch_version(base: &str) -> Option<String> {
    let res = reqwest::get(format!("{base}/{CHANNEL}")).await.ok()?;
    // reqwest only errors on transport failures, so a 4xx/5xx still arrives as
    // Ok. Without this, a digit-leading error body (e.g. "404 Not Found") would
    // pass the guard below, be taken as the version, and defeat the fallback.
    if !res.status().is_success() {
        info!("grok: {base}/{CHANNEL} returned {}", res.status());
        return None;
    }
    let text = res.text().await.ok()?.trim().to_string();
    // Guard against an error page / redirect body sneaking through as a version.
    if text.chars().next()?.is_ascii_digit() {
        Some(text)
    } else {
        info!("grok: unexpected version response from {base}: {text}");
        None
    }
}

/// An exact `=X.Y.Z` pin (what a bare `grok@X.Y.Z` normalizes to). Returns the
/// version if the request pins exactly one release; None for ranges/partials,
/// which we can't resolve without a version index xAI doesn't publish.
fn exact_version(req: &str) -> Option<String> {
    let s = req.strip_prefix('=')?;
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        Some(s.to_string())
    } else {
        None
    }
}

async fn get_grok_downloads(target: &crate::target::Target, pin: Option<String>) -> Vec<Download> {
    let Some(platform) = platform_string(target) else {
        debug!("grok: unsupported platform");
        return vec![];
    };

    // Mirror install.sh: try x.ai, fall back to the public GCS bucket. Whichever
    // host answers the version probe is the one we pull the binary from too.
    let latest = match fetch_version(BASE_URL_PRIMARY).await {
        Some(v) => Some((BASE_URL_PRIMARY, v)),
        None => fetch_version(BASE_URL_FALLBACK).await.map(|v| {
            info!("grok: {BASE_URL_PRIMARY} unreachable, using GCS fallback");
            (BASE_URL_FALLBACK, v)
        }),
    };

    let exact = pin.as_deref().and_then(exact_version);

    // The probe only discovers "latest", but a pinned artifact URL is fully
    // deterministic — so an exact `grok@X.Y.Z` can still resolve even when both
    // channel probes fail (outage / blocked endpoint). Fall back to the primary
    // host for that case. If neither the probe nor a pin gives us anything,
    // there's nothing to offer.
    if latest.is_none() && exact.is_none() {
        info!("grok: failed to resolve latest version and no exact pin given");
        return vec![];
    }
    let base = latest.as_ref().map(|(b, _)| *b).unwrap_or(BASE_URL_PRIMARY);

    let suffix = if target.os == Os::Windows { ".exe" } else { "" };
    let make = |version: &str| Download {
        download_url: format!("{base}/grok-{version}-{platform}{suffix}"),
        version: GgVersion::new(version),
        os: Some(Os::Any),
        arch: Some(Arch::Any),
        variant: Some(Variant::Any),
        tags: Default::default(),
    };

    let mut downloads = vec![];
    if let Some((_, latest)) = &latest {
        downloads.push(make(latest));
    }
    // Offer the pinned build too (gg's matcher picks whichever satisfies the
    // request); skip if it duplicates the latest we already added.
    if let Some(exact) = &exact {
        if latest.as_ref().map(|(_, l)| l != exact).unwrap_or(true) {
            downloads.push(make(exact));
        }
    }
    downloads
}

impl Executor for Grok {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        let pin = self.executor_cmd.version.as_ref().map(|v| v.to_string());
        Box::pin(async move { get_grok_downloads(&input.target, pin).await })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        vec![BinPattern::Exact(
            match &input.target.os {
                Os::Windows => "grok.exe",
                _ => "grok",
            }
            .to_string(),
        )]
    }

    fn get_name(&self) -> &str {
        "grok"
    }

    fn get_bin_dirs(&self) -> Vec<String> {
        vec![".".to_string()]
    }

    fn post_prep(&self, cache_path: &str) {
        // The binary downloads under its versioned URL basename
        // (`grok-<version>-<platform>[.exe]`); rename it to a stable `grok[.exe]`
        // so get_bins can resolve it with an exact match across runs.
        let cache = std::path::Path::new(cache_path);
        let Ok(entries) = std::fs::read_dir(cache) else {
            return;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if !name.starts_with("grok-") {
                continue;
            }
            let canonical = if name.ends_with(".exe") {
                "grok.exe"
            } else {
                "grok"
            };
            let dest = cache.join(canonical);
            if let Err(e) = std::fs::rename(entry.path(), &dest) {
                warn!("grok: failed to rename {name} to {canonical}: {e}");
                return;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                if let Err(e) = std::fs::set_permissions(&dest, perms) {
                    warn!("grok: failed to set executable permission: {e}");
                }
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn exact_version_accepts_pins_and_rejects_ranges() {
        // A bare `grok@X.Y.Z` normalizes to `=X.Y.Z` (see GgVersionReq::new).
        assert_eq!(exact_version("=0.2.50"), Some("0.2.50".to_string()));
        assert_eq!(exact_version("=10.0.1"), Some("10.0.1".to_string()));

        // Ranges/partials aren't resolvable without a version index.
        assert_eq!(exact_version("~0.1"), None); // `grok@0.1`
        assert_eq!(exact_version("^0.2.0"), None); // `grok@^0.2.0`
        assert_eq!(exact_version("=0.2.50-beta"), None); // pre-release suffix
        assert_eq!(exact_version("=0.2."), None); // empty part
        assert_eq!(exact_version("=0.2"), None); // too few parts
        assert_eq!(exact_version("=0.2.50.1"), None); // too many parts
        assert_eq!(exact_version("0.2.50"), None); // must be normalized with `=`
    }

    fn test_grok() -> Grok {
        Grok {
            executor_cmd: ExecutorCmd {
                cmd: "grok".to_string(),
                version: None,
                distribution: None,
                include_tags: HashSet::new(),
                exclude_tags: HashSet::new(),
                gems: None,
            },
        }
    }

    #[test]
    fn post_prep_renames_versioned_binary_to_canonical() {
        let dir = tempfile::tempdir().unwrap();
        // Simulate what the download leaves behind: the versioned artifact plus
        // the meta file that must NOT be touched.
        std::fs::write(dir.path().join("grok-0.2.77-linux-x86_64"), b"binary").unwrap();
        std::fs::write(dir.path().join("gg-meta.json"), b"{}").unwrap();

        test_grok().post_prep(dir.path().to_str().unwrap());

        assert!(dir.path().join("grok").exists(), "renamed to canonical name");
        assert!(
            !dir.path().join("grok-0.2.77-linux-x86_64").exists(),
            "original versioned name is gone"
        );
        assert!(dir.path().join("gg-meta.json").exists(), "meta untouched");
    }

    #[test]
    fn post_prep_preserves_exe_suffix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("grok-0.2.77-windows-x86_64.exe"), b"binary").unwrap();

        test_grok().post_prep(dir.path().to_str().unwrap());

        assert!(dir.path().join("grok.exe").exists(), "keeps .exe suffix");
    }
}
