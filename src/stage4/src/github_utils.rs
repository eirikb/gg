use std::env;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use log::debug;

use crate::target::{Arch, Os};

const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
const PUBLIC_PROXY_URL: &str = "https://ghapi.ggcmd.io";

fn first_nonempty(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

fn token_from_env() -> Option<String> {
    first_nonempty(
        ["GG_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]
            .iter()
            .map(|key| env::var(key).ok()),
    )
}

fn token_from_gh_cli() -> Option<String> {
    // Own thread - a stuck keyring should not be able to hang gg forever
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Command::new("gh").args(["auth", "token"]).output());
    });
    let output = rx.recv_timeout(Duration::from_secs(5)).ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    first_nonempty([String::from_utf8(output.stdout).ok()])
}

fn github_token() -> Option<&'static str> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            if let Some(token) = token_from_env() {
                debug!("Using GitHub token from environment");
                return Some(token);
            }
            if let Some(token) = token_from_gh_cli() {
                debug!("Using GitHub token from gh CLI");
                return Some(token);
            }
            debug!("No GitHub token found, using anonymous API access");
            None
        })
        .as_deref()
}

fn normalize_base_url(raw: Option<String>) -> String {
    raw.map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_GITHUB_API_URL.to_string())
}

fn is_default_github_host(base_url: &str) -> bool {
    base_url.eq_ignore_ascii_case(DEFAULT_GITHUB_API_URL)
}

fn hack_enabled() -> bool {
    // Shh don't tell anyone (routes via the public proxy - on purpose only
    // the rate-limit hint mentions it, keep it out of the README)
    env::var("GG_GITHUB_API_HACK").is_ok_and(|v| {
        matches!(
            v.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub fn github_api_base_url() -> String {
    // An explicit GG_GITHUB_API_URL always wins over the hack switch.
    if let Some(url) = first_nonempty([env::var("GG_GITHUB_API_URL").ok()]) {
        return normalize_base_url(Some(url));
    }
    if hack_enabled() {
        return PUBLIC_PROXY_URL.to_string();
    }
    DEFAULT_GITHUB_API_URL.to_string()
}

pub fn create_github_client() -> Result<octocrab::Octocrab, octocrab::Error> {
    let base_url = github_api_base_url();
    let mut builder = match octocrab::Octocrab::builder().base_uri(&base_url) {
        Ok(builder) => builder,
        Err(e) => {
            eprintln!("Invalid GitHub API URL '{base_url}' (set via GG_GITHUB_API_URL): {e}");
            return Err(e);
        }
    };
    // Only GG_GITHUB_TOKEN follows a custom URL. GITHUB_TOKEN/GH_TOKEN are
    // often just lying around (CI etc.), and the gh token is the user's own
    // login - not sending those anywhere but github.com
    let token = if is_default_github_host(&base_url) {
        github_token().map(|t| t.to_string())
    } else {
        let token = first_nonempty([env::var("GG_GITHUB_TOKEN").ok()]);
        if token.is_some() && !base_url.starts_with("https://") {
            eprintln!(
                "Warning: sending GG_GITHUB_TOKEN to non-HTTPS endpoint {base_url} - the token travels unencrypted"
            );
        }
        token
    };
    if let Some(token) = token {
        builder = builder.personal_token(token);
    }
    builder.build()
}

/// Turn an octocrab error into a message that tells the user what to do
/// about it, instead of the generic "Did not find any download URL!".
pub fn explain_github_error(err: &octocrab::Error) -> String {
    let base_url = github_api_base_url();
    match err {
        octocrab::Error::GitHub { source, .. } => {
            let status = source.status_code.as_u16();
            if status == 429 || source.message.to_lowercase().contains("rate limit") {
                if is_default_github_host(&base_url) {
                    format!(
                        "GitHub API rate limit exceeded ({base_url}).\n\
                         To fix, either:\n\
                         \x20 - set GITHUB_TOKEN (or GG_GITHUB_TOKEN / GH_TOKEN)\n\
                         \x20 - log in with the GitHub CLI: gh auth login\n\
                         \x20 - ...or, between you and me: GG_GITHUB_API_HACK=1 ;)"
                    )
                } else {
                    // Only GG_GITHUB_TOKEN goes to a custom endpoint, no
                    // point suggesting tokens that would be ignored
                    format!(
                        "GitHub API rate limit exceeded ({base_url}).\n\
                         To fix, either:\n\
                         \x20 - set GG_GITHUB_TOKEN (the only token sent to a custom endpoint)\n\
                         \x20 - unset GG_GITHUB_API_URL / GG_GITHUB_API_HACK to use api.github.com\n\
                         \x20   with GITHUB_TOKEN or gh auth login"
                    )
                }
            } else if status == 401 {
                format!(
                    "GitHub API rejected the token ({base_url}): {}\n\
                     The token from GG_GITHUB_TOKEN/GITHUB_TOKEN/GH_TOKEN (or the gh CLI) looks\n\
                     invalid or expired - refresh it, or unset it to use anonymous access.",
                    source.message
                )
            } else {
                format!("GitHub API error from {base_url}: {}", source.message)
            }
        }
        octocrab::Error::Service { .. } | octocrab::Error::Hyper { .. } => {
            format!(
                "Could not reach {base_url} - check your network or proxy settings.\n\
                 You can point gg at a different GitHub API endpoint with GG_GITHUB_API_URL."
            )
        }
        _ => format!("GitHub API request failed: {err}"),
    }
}

/// Failures stashed per repo (executors run concurrently) so the final "no
/// download URL" error can say what happened. Can't print right away - the
/// progress bars own stderr and would just eat it.
static GITHUB_ERRORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn record_github_error(scope: &str, err: &octocrab::Error) {
    debug!("GitHub API error for {scope}: {err}");
    let msg = format!("[{scope}] {}", explain_github_error(err));
    let mut errors = GITHUB_ERRORS.lock().unwrap();
    if !errors.contains(&msg) {
        errors.push(msg);
    }
}

pub fn take_github_errors() -> Vec<String> {
    std::mem::take(&mut *GITHUB_ERRORS.lock().unwrap())
}

pub fn detect_os_from_name(name: &str) -> Option<Os> {
    let name_lower = name.to_lowercase();
    // Android assets carry "linux" too (bun-linux-x64-android-*), and we don't
    // target Android. Delimited token only, or we'd drop desktop tools that
    // just have "android" somewhere in the name.
    if name_lower
        .split(|c: char| c == '-' || c == '_' || c == '.')
        .any(|part| part == "android")
    {
        return None;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_nonempty_picks_first_set_value() {
        assert_eq!(
            first_nonempty([None, Some("  ".to_string()), Some("tok".to_string())]),
            Some("tok".to_string())
        );
        assert_eq!(first_nonempty([None, None]), None);
        assert_eq!(first_nonempty([Some("".to_string())]), None);
    }

    #[test]
    fn test_first_nonempty_trims_whitespace() {
        assert_eq!(
            first_nonempty([Some("  token\n".to_string())]),
            Some("token".to_string())
        );
    }

    #[test]
    fn test_normalize_base_url_defaults() {
        assert_eq!(normalize_base_url(None), DEFAULT_GITHUB_API_URL);
        assert_eq!(
            normalize_base_url(Some("".to_string())),
            DEFAULT_GITHUB_API_URL
        );
        assert_eq!(
            normalize_base_url(Some("  ".to_string())),
            DEFAULT_GITHUB_API_URL
        );
    }

    #[test]
    fn test_normalize_base_url_strips_trailing_slashes() {
        assert_eq!(
            normalize_base_url(Some("https://api.github.com/".to_string())),
            "https://api.github.com"
        );
        assert_eq!(
            normalize_base_url(Some("https://ghapi.ggcmd.io//".to_string())),
            "https://ghapi.ggcmd.io"
        );
    }

    #[test]
    fn test_detect_os_rejects_android() {
        // darwin/windows too, not just linux - guard beats every OS keyword
        assert_eq!(
            detect_os_from_name("bun-linux-x64-android-baseline-profile.zip"),
            None
        );
        assert_eq!(detect_os_from_name("bun-darwin-x64-android.zip"), None);
        assert_eq!(detect_os_from_name("bun-windows-x64-android.zip"), None);
    }

    #[test]
    fn test_detect_os_keeps_desktop_linux() {
        assert_eq!(detect_os_from_name("bun-linux-x64.zip"), Some(Os::Linux));
        assert_eq!(
            detect_os_from_name("bun-linux-x64-baseline.zip"),
            Some(Os::Linux)
        );
        // "android" fused into a bigger token shouldn't trip the guard
        assert_eq!(
            detect_os_from_name("mytool-linux-x64-noandroidhere.zip"),
            Some(Os::Linux)
        );
    }

    #[test]
    fn test_is_default_github_host() {
        assert!(is_default_github_host("https://api.github.com"));
        assert!(is_default_github_host("https://API.github.com"));
        // trailing slash is stripped by normalize_base_url before this check
        assert!(is_default_github_host(&normalize_base_url(Some(
            "https://api.github.com/".to_string()
        ))));
        assert!(!is_default_github_host("https://ghapi.ggcmd.io"));
    }
}
