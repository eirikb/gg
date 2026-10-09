use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use crate::executor::{
    find_jar_file, AppInput, AppPath, BinPattern, Download, Executor, ExecutorCmd, ExecutorDep,
    GgMeta, GgVersion,
};
use crate::github_utils::{
    create_github_client, detect_arch_from_name, detect_os_from_name, record_github_error,
};
use crate::target::Os::Windows;
use crate::target::{Arch, Os, Variant};
use log::{debug, warn};

pub struct GitHub {
    pub executor_cmd: ExecutorCmd,
    pub owner: String,
    pub repo: String,
    pub predefined_deps: Option<Vec<ExecutorDep>>,
    pub predefined_bins: Option<Vec<String>>,
    pub excluded_asset_keywords: Vec<String>,
}

impl GitHub {
    pub fn new(executor_cmd: ExecutorCmd, owner: String, repo: String) -> Self {
        Self {
            executor_cmd,
            owner,
            repo,
            predefined_deps: None,
            predefined_bins: None,
            excluded_asset_keywords: vec![],
        }
    }

    pub fn new_with_config(
        executor_cmd: ExecutorCmd,
        owner: String,
        repo: String,
        predefined_deps: Option<Vec<ExecutorDep>>,
        predefined_bins: Option<Vec<String>>,
    ) -> Self {
        Self {
            executor_cmd,
            owner,
            repo,
            predefined_deps,
            predefined_bins,
            excluded_asset_keywords: vec![],
        }
    }

    /// Skip assets whose name contains any of these keywords. For repos that
    /// publish several same-platform variants per release (kimi-cli's
    /// `-onedir` bundles, mistral-vibe's `vibe-acp-*` editor-protocol
    /// builds), the os+arch+version sort ties and selection becomes
    /// arbitrary; excluding the unwanted variants keeps it deterministic.
    pub fn with_excluded_asset_keywords(mut self, keywords: Vec<&str>) -> Self {
        self.excluded_asset_keywords = keywords.into_iter().map(|k| k.to_lowercase()).collect();
        self
    }

    fn is_excluded_asset(name: &str, excluded_keywords: &[String]) -> bool {
        let name_lower = name.to_lowercase();
        excluded_keywords.iter().any(|k| name_lower.contains(k))
    }

    async fn detect_language_and_deps(&self) -> Vec<ExecutorDep> {
        let octocrab = create_github_client().unwrap();

        let repo_result = octocrab.repos(&self.owner, &self.repo).get().await;
        if let Err(err) = &repo_result {
            record_github_error(&format!("{}/{}", self.owner, self.repo), err);
        }
        if let Ok(repo_info) = repo_result {
            if let Some(language) = repo_info.language {
                let language_str = language.as_str().unwrap_or("").to_lowercase();
                return match language_str.as_str() {
                    "java" | "kotlin" | "scala" | "clojure" => {
                        vec![ExecutorDep::new("java".to_string(), None)]
                    }
                    "javascript" | "typescript" => vec![ExecutorDep::new("node".to_string(), None)],
                    "go" => vec![],
                    "rust" => vec![],
                    "c" | "c++" | "cpp" => vec![],
                    _ => vec![],
                };
            }
        }

        vec![]
    }

    fn is_likely_binary(name: &str) -> bool {
        let name_lower = name.to_lowercase();

        if name_lower.ends_with(".msi") {
            return false;
        }

        // Checksum/signature/metadata companions (e.g. deno publishes
        // deno-x86_64-unknown-linux-gnu.sha256sum next to the .zip). These
        // would otherwise pass the os+arch heuristic below.
        let metadata_suffixes = [
            ".sha256sum",
            ".sha512sum",
            ".sha1sum",
            ".md5sum",
            ".sha256",
            ".sha512",
            ".sha1",
            ".md5",
            ".sum",
            ".sig",
            ".asc",
            ".sigstore",
            ".minisig",
            ".pem",
            ".sbom",
            ".json",
            ".jsonl",
            ".txt",
            ".md",
        ];
        if metadata_suffixes
            .iter()
            .any(|ext| name_lower.ends_with(ext))
        {
            return false;
        }

        if name_lower.contains(".orig.tar")
            || name_lower.contains("-src.")
            || name_lower.contains("_src.")
            || name_lower.contains("-source.")
            || name_lower.contains("_source.")
        {
            return false;
        }

        if name_lower.contains("i686") || name_lower.contains("i386") || name_lower.contains("ia32")
        {
            return false;
        }

        let binary_extensions = [
            ".exe", ".zip", ".tar.gz", ".tgz", ".tar.bz2", ".7z", ".gem", ".jar",
        ];

        for ext in &binary_extensions {
            if name_lower.contains(ext) {
                return true;
            }
        }

        if (name_lower.contains("linux")
            || name_lower.contains("darwin")
            || name_lower.contains("macos")
            || name_lower.contains("windows"))
            && (name_lower.contains("x64")
                || name_lower.contains("x86")
                || name_lower.contains("arm64")
                || name_lower.contains("aarch64"))
        {
            return true;
        }

        false
    }

    // A jar-only release (RepoSense) runs via `java -jar`. A jar next to a real
    // binary doesn't count - the binary wins, no forced java. #244
    fn should_run_as_jar(&self, app_path: &AppPath) -> bool {
        if self.predefined_bins.is_some() || find_jar_file(app_path).is_none() {
            return false;
        }
        let names = [
            self.repo.clone(),
            self.repo.to_lowercase(),
            format!("{}.exe", self.repo),
            format!("{}.exe", self.repo.to_lowercase()),
        ];
        let has_native_binary = ["bin", "."].iter().any(|sub| {
            names
                .iter()
                .any(|n| app_path.install_dir.join(sub).join(n).is_file())
        });
        !has_native_binary
    }

    // Where a bare-binary download gets moved to, so the exact bin patterns hit.
    // Can be nested (fortio: bin/fortio).
    fn canonical_bin_name(&self, exe: bool) -> String {
        let predefined = self
            .predefined_bins
            .iter()
            .flatten()
            .find(|b| b.to_lowercase().ends_with(".exe") == exe);
        match predefined {
            Some(bin) => bin.clone(),
            None if exe => format!("{}.exe", self.repo),
            None => self.repo.clone(),
        }
    }
}

// ELF, Mach-O (thin and fat) or a script with a shebang
fn is_native_executable(path: &std::path::Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.read_exact(&mut magic).is_err() {
        return false;
    }
    matches!(
        magic,
        [0x7f, b'E', b'L', b'F']
            | [0xfe, 0xed, 0xfa, 0xce]
            | [0xfe, 0xed, 0xfa, 0xcf]
            | [0xce, 0xfa, 0xed, 0xfe]
            | [0xcf, 0xfa, 0xed, 0xfe]
            | [0xca, 0xfe, 0xba, 0xbe]
    ) || magic.starts_with(b"#!")
}

/// Prefer the host's libc variant, falling back to the other when it's the only
/// one for a given os/arch/version (so a musl-only static build still resolves).
fn prefer_libc_variant(downloads: Vec<Download>, prefer_musl: bool) -> Vec<Download> {
    let key = |d: &Download| format!("{:?}|{:?}|{:?}", d.os, d.arch, d.version);
    let mut has_preferred: HashSet<String> = HashSet::new();
    for d in &downloads {
        let is_musl = d.variant == Some(Variant::Musl);
        if is_musl == prefer_musl {
            has_preferred.insert(key(d));
        }
    }
    downloads
        .into_iter()
        .filter(|d| {
            let is_musl = d.variant == Some(Variant::Musl);
            is_musl == prefer_musl || !has_preferred.contains(&key(d))
        })
        // Normalise to Any so the shared matcher doesn't re-drop a musl fallback.
        .map(|mut d| {
            d.variant = Some(Variant::Any);
            d
        })
        .collect()
}

impl Executor for GitHub {
    fn get_executor_cmd(&self) -> &ExecutorCmd {
        &self.executor_cmd
    }

    fn get_download_urls<'a>(
        &self,
        input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<Download>> + 'a>> {
        let owner = self.owner.clone();
        let repo = self.repo.clone();
        let excluded_keywords = self.excluded_asset_keywords.clone();
        let prefer_musl = input.target.variant == Some(Variant::Musl);

        Box::pin(async move {
            let mut downloads: Vec<Download> = vec![];
            let octocrab = create_github_client().expect("Failed to create GitHub API client");

            let mut page: u32 = 1;
            loop {
                let releases_result = octocrab
                    .repos(&owner, &repo)
                    .releases()
                    .list()
                    .page(page)
                    .per_page(100)
                    .send()
                    .await;

                match releases_result {
                    Ok(releases) => {
                        for release in releases.items {
                            for asset in release.assets {
                                if !Self::is_likely_binary(&asset.name) {
                                    continue;
                                }

                                if Self::is_excluded_asset(&asset.name, &excluded_keywords) {
                                    debug!("Skipping excluded asset: {}", asset.name);
                                    continue;
                                }

                                let os = detect_os_from_name(&asset.name);
                                let arch = detect_arch_from_name(&asset.name);

                                debug!("Asset: {} -> OS: {:?}, Arch: {:?}", asset.name, os, arch);

                                if (os.is_some() && arch.is_some())
                                    || (os.is_none() && arch.is_none()
                                        || (os == Some(Os::Windows) && arch.is_none()))
                                {
                                    debug!(
                                        "Adding download: {} with OS: {:?}, Arch: {:?}",
                                        asset.browser_download_url, os, arch
                                    );
                                    let is_musl = asset.name.to_lowercase().contains("musl")
                                        && !matches!(os, Some(Os::Windows) | Some(Os::Mac));
                                    let variant = if is_musl {
                                        Some(Variant::Musl)
                                    } else {
                                        Some(Variant::Any)
                                    };
                                    downloads.push(Download {
                                        download_url: asset.browser_download_url.to_string(),
                                        version: GgVersion::new(release.tag_name.as_str()),
                                        os: os.or(Some(Os::Any)),
                                        arch: arch.or(Some(Arch::Any)),
                                        tags: HashSet::new(),
                                        variant,
                                    });
                                }
                            }
                        }

                        if releases.next.is_none() {
                            break;
                        }
                        page += 1;
                    }
                    Err(err) => {
                        record_github_error(&format!("{owner}/{repo}"), &err);
                        break;
                    }
                }
            }
            debug!("Total downloads found: {}", downloads.len());
            prefer_libc_variant(downloads, prefer_musl)
        })
    }

    fn get_bins(&self, input: &AppInput) -> Vec<BinPattern> {
        if let Some(predefined_bins) = &self.predefined_bins {
            return predefined_bins
                .iter()
                .map(|s| BinPattern::Exact(s.clone()))
                .collect();
        }

        let base_name = &self.repo;

        let mut patterns = vec![
            BinPattern::Exact(match &input.target.os {
                Windows => format!("{}.exe", base_name),
                _ => base_name.clone(),
            }),
            BinPattern::Exact(match &input.target.os {
                Windows => format!("{}.exe", base_name.to_lowercase()),
                _ => base_name.to_lowercase(),
            }),
            BinPattern::Exact(base_name.clone()),
            BinPattern::Exact(base_name.to_lowercase()),
        ];

        if input.target.os == Windows {
            patterns.push(BinPattern::Regex(r".*\.exe$".to_string()));
        }

        patterns.push(BinPattern::Regex(r"^[^.]*$".to_string()));

        patterns
    }

    fn get_bins_for_path(&self, input: &AppInput, app_path: &AppPath) -> Vec<BinPattern> {
        // For a jar run, resolve `java` directly - the generic patterns match a
        // JDK helper (jimage, jabswitch, jar, ...) from the java dep's bin dir
        // before java. #244
        if self.should_run_as_jar(app_path) {
            return vec![BinPattern::Exact("java".to_string())];
        }
        self.get_bins(input)
    }

    fn cached_install_is_valid(&self, app_path: &AppPath) -> bool {
        self.should_run_as_jar(app_path)
    }

    fn get_name(&self) -> &str {
        &self.repo
    }

    fn post_prep(&self, cache_path: &str) {
        // A release asset that is the executable itself (herdr-linux-x86_64), not
        // an archive: nothing gave it an exec bit, and its name matches no bin
        // pattern. Fix both.
        let cache = std::path::Path::new(cache_path);
        let Ok(entries) = std::fs::read_dir(cache) else {
            return;
        };
        let mut files = entries
            .flatten()
            .filter(|e| e.file_name() != "gg-meta.json")
            .map(|e| e.path());
        let (Some(file), None) = (files.next(), files.next()) else {
            return;
        };
        // An archive can unpack to a single file too. That one already has its
        // real name and mode, so only touch the file that is the download itself.
        let asset = std::fs::read_to_string(cache.join("gg-meta.json"))
            .ok()
            .and_then(|json| serde_json::from_str::<GgMeta>(&json).ok())
            .and_then(|meta| reqwest::Url::parse(&meta.download.download_url).ok())
            .and_then(|url| url.path_segments()?.next_back().map(str::to_string));
        if !file.is_file() || file.file_name().and_then(|n| n.to_str()) != asset.as_deref() {
            return;
        }
        let has_ext = |wanted: &str| {
            file.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
        };
        let exe = has_ext("exe");
        // A self-executing jar has a shebang, but runs via java -jar
        if has_ext("jar") || (!exe && !is_native_executable(&file)) {
            return;
        }
        let dest = cache.join(self.canonical_bin_name(exe));
        if file != dest {
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::rename(&file, &dest) {
                warn!(
                    "{}: failed to rename {:?} to {:?}: {e}",
                    self.repo, file, dest
                );
                return;
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            if let Err(e) = std::fs::set_permissions(&dest, perms) {
                warn!("{}: failed to set executable permission: {e}", self.repo);
            }
        }
    }

    fn customize_args(&self, input: &AppInput, app_path: &AppPath) -> Vec<String> {
        if self.should_run_as_jar(app_path) {
            if let Some(jar_name) = find_jar_file(app_path) {
                if let Some(jar_path) = app_path.install_dir.join(&jar_name).to_str() {
                    let mut args = vec!["-jar".to_string(), jar_path.to_string()];
                    args.extend_from_slice(&input.app_args);
                    return args;
                }
            }
        }
        input.app_args.clone()
    }

    fn get_deps<'a>(
        &'a self,
        _input: &'a AppInput,
    ) -> Pin<Box<dyn Future<Output = Vec<ExecutorDep>> + 'a>> {
        Box::pin(async move {
            if let Some(predefined_deps) = &self.predefined_deps {
                return predefined_deps.clone();
            }
            self.detect_language_and_deps().await
        })
    }

    fn get_env(&self, app_path: &AppPath) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Ok, so, if this tool uses gems (has .gem files), set gem environment
        let gem_home = app_path.install_dir.join("gem_home");
        if gem_home.exists() {
            let gem_home_str = gem_home.to_string_lossy().to_string();
            env.insert("GEM_HOME".to_string(), gem_home_str.clone());
            env.insert("GEM_PATH".to_string(), gem_home_str);
        }

        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_binary_accepts_binaries() {
        assert!(GitHub::is_likely_binary("tool-linux-amd64.tar.gz"));
        assert!(GitHub::is_likely_binary("tool-windows-x64.zip"));
        assert!(GitHub::is_likely_binary("tool.exe"));
        assert!(GitHub::is_likely_binary("tool-darwin-arm64.tgz"));
        assert!(GitHub::is_likely_binary("tool-v1.0.0-linux-x86_64.tar.bz2"));
        assert!(GitHub::is_likely_binary("tool.jar"));
        assert!(GitHub::is_likely_binary("tool.gem"));
        assert!(GitHub::is_likely_binary("fortio_win_1.73.0.zip"));
    }

    #[test]
    fn test_is_likely_binary_rejects_source_tarballs() {
        assert!(!GitHub::is_likely_binary("fortio_1.73.0.orig.tar.gz"));
        assert!(!GitHub::is_likely_binary("package_1.0.0.orig.tar.xz"));

        assert!(!GitHub::is_likely_binary("tool-1.0.0-src.tar.gz"));
        assert!(!GitHub::is_likely_binary("tool_1.0.0_src.zip"));
        assert!(!GitHub::is_likely_binary("tool-source.tar.gz"));
        assert!(!GitHub::is_likely_binary("tool_source.zip"));
    }

    #[test]
    fn test_is_likely_binary_rejects_32bit_x86() {
        assert!(!GitHub::is_likely_binary("uv-i686-pc-windows-msvc.zip"));
        assert!(!GitHub::is_likely_binary("tool-i386-linux.tar.gz"));
        assert!(!GitHub::is_likely_binary("tool-ia32.zip"));
        assert!(!GitHub::is_likely_binary("node-v20.0.0-win-ia32.zip"));

        assert!(GitHub::is_likely_binary("uv-x86_64-pc-windows-msvc.zip"));
        assert!(GitHub::is_likely_binary("tool-v1.0.0-linux-x86_64.tar.bz2"));
    }

    #[test]
    fn test_is_likely_binary_rejects_msi() {
        assert!(!GitHub::is_likely_binary("tool-setup.msi"));
        assert!(!GitHub::is_likely_binary("Tool-1.0.0-x64.msi"));
    }

    #[test]
    fn test_is_likely_binary_rejects_non_binaries() {
        assert!(!GitHub::is_likely_binary("checksums.txt"));
        assert!(!GitHub::is_likely_binary("SHA256SUMS"));
        assert!(!GitHub::is_likely_binary("tool.asc"));
        assert!(!GitHub::is_likely_binary("CHANGELOG.md"));
    }

    #[test]
    fn test_is_excluded_asset() {
        // kimi-cli ships a single-binary and an -onedir bundle per platform
        let excluded = vec!["onedir".to_string()];
        assert!(GitHub::is_excluded_asset(
            "kimi-1.47.0-x86_64-unknown-linux-gnu-onedir.tar.gz",
            &excluded
        ));
        assert!(!GitHub::is_excluded_asset(
            "kimi-1.47.0-x86_64-unknown-linux-gnu.tar.gz",
            &excluded
        ));

        // mistral-vibe ships vibe-acp-* (editor protocol) next to vibe-*
        let excluded = vec!["vibe-acp".to_string()];
        assert!(GitHub::is_excluded_asset(
            "vibe-acp-linux-x86_64-2.14.1.zip",
            &excluded
        ));
        assert!(!GitHub::is_excluded_asset(
            "vibe-linux-x86_64-2.14.1.zip",
            &excluded
        ));

        // matching is case-insensitive on the asset name
        assert!(GitHub::is_excluded_asset(
            "Tool-Linux-OneDir.tar.gz",
            &["onedir".to_string()]
        ));

        assert!(!GitHub::is_excluded_asset("anything.tar.gz", &[]));
    }

    fn dl(url: &str, variant: Variant, version: &str) -> Download {
        Download {
            download_url: url.to_string(),
            version: GgVersion::new(version),
            os: Some(Os::Linux),
            arch: Some(Arch::X86_64),
            tags: HashSet::new(),
            variant: Some(variant),
        }
    }

    fn urls(downloads: Vec<Download>) -> Vec<String> {
        let mut u: Vec<String> = downloads.into_iter().map(|d| d.download_url).collect();
        u.sort();
        u
    }

    #[test]
    fn test_prefer_libc_glibc_host_drops_musl_when_glibc_exists() {
        let downloads = vec![
            dl("pnpm-linux-x64.tar.gz", Variant::Any, "11.9.0"),
            dl("pnpm-linux-x64-musl.tar.gz", Variant::Musl, "11.9.0"),
        ];
        assert_eq!(
            urls(prefer_libc_variant(downloads, false)),
            vec!["pnpm-linux-x64.tar.gz"]
        );
    }

    #[test]
    fn test_prefer_libc_glibc_host_keeps_musl_only_build() {
        let downloads = vec![dl("tool-linux-x64-musl.tar.gz", Variant::Musl, "1.0.0")];
        let result = prefer_libc_variant(downloads, false);
        assert_eq!(
            result
                .iter()
                .map(|d| d.download_url.clone())
                .collect::<Vec<_>>(),
            vec!["tool-linux-x64-musl.tar.gz"]
        );
        // Re-tagged Any so the shared matcher won't hard-drop the fallback.
        assert_eq!(result[0].variant, Some(Variant::Any));
    }

    #[test]
    fn test_prefer_libc_musl_host_drops_glibc_when_musl_exists() {
        let downloads = vec![
            dl("pnpm-linux-x64.tar.gz", Variant::Any, "11.9.0"),
            dl("pnpm-linux-x64-musl.tar.gz", Variant::Musl, "11.9.0"),
        ];
        assert_eq!(
            urls(prefer_libc_variant(downloads, true)),
            vec!["pnpm-linux-x64-musl.tar.gz"]
        );
    }

    #[test]
    fn test_prefer_libc_keeps_both_when_versions_differ() {
        let downloads = vec![
            dl("a-linux-x64.tar.gz", Variant::Any, "2.0.0"),
            dl("b-linux-x64-musl.tar.gz", Variant::Musl, "1.0.0"),
        ];
        assert_eq!(
            urls(prefer_libc_variant(downloads, false)),
            vec!["a-linux-x64.tar.gz", "b-linux-x64-musl.tar.gz"]
        );
    }

    #[test]
    fn test_is_likely_binary_rejects_checksum_companions() {
        // Real-world: deno publishes these next to each platform zip,
        // and they used to pass the os+arch heuristic
        assert!(!GitHub::is_likely_binary(
            "deno-x86_64-unknown-linux-gnu.sha256sum"
        ));
        assert!(!GitHub::is_likely_binary(
            "deno-x86_64-pc-windows-msvc.sha256sum"
        ));
        assert!(!GitHub::is_likely_binary("tool-linux-x64.zip.sha256"));
        assert!(!GitHub::is_likely_binary("tool-darwin-arm64.tar.gz.sig"));
        assert!(!GitHub::is_likely_binary("tool-windows-x64.zip.asc"));
        assert!(!GitHub::is_likely_binary("tool-linux-arm64.tgz.minisig"));
        assert!(!GitHub::is_likely_binary("tool-linux-x64.sbom"));
        assert!(!GitHub::is_likely_binary("tool-macos-arm64.spdx.json"));
        assert!(!GitHub::is_likely_binary("tool-linux-x64.intoto.jsonl"));
        assert!(!GitHub::is_likely_binary("tool-linux-x64.zip.md5"));

        // ...but the actual archives still pass
        assert!(GitHub::is_likely_binary(
            "deno-x86_64-unknown-linux-gnu.zip"
        ));
        assert!(GitHub::is_likely_binary("deno-x86_64-pc-windows-msvc.zip"));
    }

    #[test]
    fn test_should_run_as_jar() {
        use std::fs::File;
        let dir = tempfile::tempdir().unwrap();
        let cmd = ExecutorCmd {
            cmd: "gh/reposense/RepoSense".to_string(),
            version: None,
            distribution: None,
            include_tags: std::collections::HashSet::new(),
            exclude_tags: std::collections::HashSet::new(),
            gems: None,
        };
        let gh = GitHub::new(cmd, "reposense".to_string(), "RepoSense".to_string());
        let app_path = AppPath {
            install_dir: dir.path().to_path_buf(),
        };

        // No jar -> not a jar run
        assert!(!gh.should_run_as_jar(&app_path));

        // Jar and no native binary -> run via java
        File::create(dir.path().join("RepoSense.jar")).unwrap();
        assert!(gh.should_run_as_jar(&app_path));

        // A native binary next to the jar -> binary wins, no forced java
        File::create(dir.path().join("RepoSense")).unwrap();
        assert!(!gh.should_run_as_jar(&app_path));
    }

    fn bare(owner: &str, repo: &str, bins: Option<Vec<&str>>) -> GitHub {
        let cmd = ExecutorCmd {
            cmd: format!("gh/{owner}/{repo}"),
            version: None,
            distribution: None,
            include_tags: std::collections::HashSet::new(),
            exclude_tags: std::collections::HashSet::new(),
            gems: None,
        };
        GitHub::new_with_config(
            cmd,
            owner.to_string(),
            repo.to_string(),
            None,
            bins.map(|b| b.into_iter().map(|s| s.to_string()).collect()),
        )
    }

    // What prep leaves behind: the unpacked files plus the meta naming the asset
    fn downloaded(asset: &str, files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, content) in files {
            std::fs::write(dir.path().join(name), content).unwrap();
        }
        let meta = serde_json::json!({
            "version_req": "*",
            "download": {
                "version": "1.0.0",
                "tags": [],
                "download_url": format!("https://github.com/o/r/releases/download/v1/{asset}"),
                "arch": "X86_64",
                "os": "Linux",
                "variant": "Any"
            },
            "cmd": {
                "cmd": "gh/o/r",
                "version": null,
                "distribution": null,
                "include_tags": [],
                "exclude_tags": [],
                "gems": null
            }
        });
        std::fs::write(dir.path().join("gg-meta.json"), meta.to_string()).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn post_prep_makes_bare_binary_runnable() {
        use std::os::unix::fs::PermissionsExt;
        let asset = "herdr-linux-x86_64";
        let dir = downloaded(asset, &[(asset, b"\x7fELF....")]);

        bare("herdrdev", "herdr", None).post_prep(dir.path().to_str().unwrap());

        let bin = dir.path().join("herdr");
        assert!(bin.exists(), "renamed to the repo name");
        assert!(!dir.path().join(asset).exists());
        assert_eq!(
            std::fs::metadata(&bin).unwrap().permissions().mode() & 0o111,
            0o111,
            "executable"
        );
        assert!(dir.path().join("gg-meta.json").exists(), "meta untouched");
    }

    #[test]
    fn post_prep_uses_predefined_bin_name() {
        let bins = Some(vec!["antigravity", "antigravity.exe"]);
        let gh = bare("google-antigravity", "antigravity-cli", bins);

        let asset = "antigravity-cli-darwin-arm64";
        let dir = downloaded(asset, &[(asset, b"#!/bin/sh\n")]);
        gh.post_prep(dir.path().to_str().unwrap());
        assert!(dir.path().join("antigravity").exists());

        let asset = "antigravity-cli-windows-x64.exe";
        let dir = downloaded(asset, &[(asset, b"MZ")]);
        gh.post_prep(dir.path().to_str().unwrap());
        assert!(dir.path().join("antigravity.exe").exists());

        // fortio's bin lives in bin/
        let asset = "fortio-linux-amd64";
        let dir = downloaded(asset, &[(asset, b"\x7fELF....")]);
        bare("fortio", "fortio", Some(vec!["bin/fortio", "fortio.exe"]))
            .post_prep(dir.path().to_str().unwrap());
        assert!(dir.path().join("bin/fortio").exists());
    }

    #[test]
    fn post_prep_leaves_everything_else_alone() {
        let untouched = |asset: &str, files: &[(&str, &[u8])]| {
            let dir = downloaded(asset, files);
            bare("x", "tool", None).post_prep(dir.path().to_str().unwrap());
            for (name, _) in files {
                assert!(dir.path().join(name).exists(), "{} was moved", name);
            }
        };

        // An archive that unpacked to one binary: argv[0] may matter (busybox)
        untouched("tool-linux.tar.gz", &[("busybox", b"\x7fELF....")]);
        untouched(
            "tool-linux.tar.gz",
            &[("a", b"\x7fELF...."), ("b", b"\x7fELF....")],
        );
        // A jar is a zip, a .deb is an ar archive
        untouched("tool.jar", &[("tool.jar", b"PK\x03\x04")]);
        untouched("tool-exec.jar", &[("tool-exec.jar", b"#!/bin/sh\n")]);
        untouched("tool_amd64.deb", &[("tool_amd64.deb", b"!<arch>\n")]);
    }
}
