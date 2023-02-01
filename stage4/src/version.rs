use std::collections::HashSet;

use semver::{Version, VersionReq};

#[derive(Debug, Clone)]
pub struct GGVersionReq {
    pub valid: bool,
    pub version_req: VersionReq,
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct GGVersion {
    pub valid: bool,
    pub version: Version,
    pub tags: HashSet<String>,
}

impl GGVersionReq {
    pub fn new(text: &str, tags: HashSet<String>) -> GGVersionReq {
        let version_req = VersionReq::parse(text);
        return GGVersionReq {
            valid: version_req.is_ok(),
            version_req: version_req.unwrap_or_default(),
            tags,
        };
    }

    pub fn matches(&self, version: &GGVersion) -> bool {
        return self.version_req.matches(&version.version);
    }
}

impl GGVersion {
    pub fn new(text: &str, tags: HashSet<String>) -> GGVersion {
        let version = Version::parse(text);
        return GGVersion {
            valid: version.is_ok(),
            version: version.unwrap_or(Version::new(0, 0, 0)),
            tags,
        };
    }
}
