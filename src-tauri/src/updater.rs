use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub published_at: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Serialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub release_url: Option<String>,
    pub download_url: Option<String>,
}

const GITHUB_API_URL: &str = "https://api.github.com/repos/ChenM0M/VibeHub/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let client = reqwest::Client::new();

    let response = client
        .get(GITHUB_API_URL)
        .header("User-Agent", "VibeHub-Updater")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    let release: ReleaseInfo = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let latest = release.tag_name.trim_start_matches('v');
    let current = CURRENT_VERSION;

    let has_update = version_is_newer(latest, current);

    // Select download URL based on platform
    let download_url = select_download_asset(&release.assets);

    Ok(UpdateCheckResult {
        has_update,
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        release_notes: if has_update { Some(release.body) } else { None },
        release_url: if has_update {
            Some(release.html_url)
        } else {
            None
        },
        download_url,
    })
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    matches!(
        compare_versions(latest, current),
        Some(std::cmp::Ordering::Greater)
    )
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left = parse_version(left)?;
    let right = parse_version(right)?;

    match left.core.cmp(&right.core) {
        std::cmp::Ordering::Equal => compare_prerelease(&left.prerelease, &right.prerelease),
        ordering => Some(ordering),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedVersion {
    core: [u64; 3],
    prerelease: Vec<PrereleaseIdentifier>,
}

#[derive(Debug, PartialEq, Eq)]
enum PrereleaseIdentifier {
    Numeric(u64),
    Text(String),
}

fn parse_version(version: &str) -> Option<ParsedVersion> {
    let trimmed = version.trim().trim_start_matches('v');
    let (core, prerelease) = match trimmed.split_once('-') {
        Some((core, prerelease)) => (core, prerelease),
        None => (trimmed, ""),
    };

    let mut core_parts = core.split('.');
    let parsed = [
        core_parts.next()?.parse().ok()?,
        core_parts.next()?.parse().ok()?,
        core_parts.next()?.parse().ok()?,
    ];

    if core_parts.next().is_some() {
        return None;
    }

    let prerelease = if prerelease.is_empty() {
        Vec::new()
    } else {
        prerelease
            .split('.')
            .map(|part| match part.parse::<u64>() {
                Ok(value) => Some(PrereleaseIdentifier::Numeric(value)),
                Err(_) if !part.is_empty() => Some(PrereleaseIdentifier::Text(part.to_string())),
                Err(_) => None,
            })
            .collect::<Option<Vec<_>>>()?
    };

    Some(ParsedVersion {
        core: parsed,
        prerelease,
    })
}

fn compare_prerelease(
    left: &[PrereleaseIdentifier],
    right: &[PrereleaseIdentifier],
) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;

    match (left.is_empty(), right.is_empty()) {
        (true, true) => return Some(Ordering::Equal),
        (true, false) => return Some(Ordering::Greater),
        (false, true) => return Some(Ordering::Less),
        (false, false) => {}
    }

    for (left_part, right_part) in left.iter().zip(right.iter()) {
        let ordering = match (left_part, right_part) {
            (PrereleaseIdentifier::Numeric(left), PrereleaseIdentifier::Numeric(right)) => {
                left.cmp(right)
            }
            (PrereleaseIdentifier::Numeric(_), PrereleaseIdentifier::Text(_)) => Ordering::Less,
            (PrereleaseIdentifier::Text(_), PrereleaseIdentifier::Numeric(_)) => Ordering::Greater,
            (PrereleaseIdentifier::Text(left), PrereleaseIdentifier::Text(right)) => {
                left.cmp(right)
            }
        };

        if ordering != Ordering::Equal {
            return Some(ordering);
        }
    }

    Some(left.len().cmp(&right.len()))
}

fn select_download_asset(assets: &[ReleaseAsset]) -> Option<String> {
    #[cfg(target_os = "windows")]
    let patterns = ["_x64-setup.exe", "_x64_en-US.msi", "x64-setup.exe", ".exe"];

    #[cfg(target_os = "macos")]
    let patterns = [".dmg", "_aarch64.dmg", "_x64.dmg"];

    #[cfg(target_os = "linux")]
    let patterns = [".AppImage", ".deb", ".tar.gz"];

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let patterns: [&str; 0] = [];

    for pattern in patterns {
        if let Some(asset) = assets.iter().find(|a| a.name.contains(pattern)) {
            return Some(asset.browser_download_url.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::version_is_newer;

    #[test]
    fn detects_stable_upgrade_over_prerelease() {
        assert!(version_is_newer("2.0.0", "2.0.0-pre.1"));
    }

    #[test]
    fn compares_prerelease_identifiers() {
        assert!(version_is_newer("2.0.0-pre.2", "2.0.0-pre.1"));
        assert!(version_is_newer("2.0.0-pre.beta", "2.0.0-pre.9"));
        assert!(!version_is_newer("2.0.0-pre.1", "2.0.0-pre.2"));
    }

    #[test]
    fn compares_core_versions_before_prerelease_rules() {
        assert!(version_is_newer("2.0.1-pre.1", "2.0.0"));
        assert!(!version_is_newer("1.3.5", "2.0.0-pre.1"));
    }
}
