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
        release_url: if has_update { Some(release.html_url) } else { None },
        download_url,
    })
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .collect()
    };
    let latest_parts = parse(latest);
    let current_parts = parse(current);
    
    for i in 0..3 {
        let l = latest_parts.get(i).unwrap_or(&0);
        let c = current_parts.get(i).unwrap_or(&0);
        if l > c { return true; }
        if l < c { return false; }
    }
    false
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
