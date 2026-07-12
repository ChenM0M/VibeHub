use serde_json::json;
use std::path::PathBuf;
use std::time::Instant;
use vibehub_core::v3::ProjectIndexService;

#[cfg(unix)]
fn peak_rss_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let rss = unsafe { usage.assume_init() }.ru_maxrss as u64;
    #[cfg(target_os = "macos")]
    return Some(rss);
    #[cfg(not(target_os = "macos"))]
    return Some(rss.saturating_mul(1024));
}

#[cfg(not(unix))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let service = ProjectIndexService::open(&root)?;
    let baseline_peak_rss_bytes = peak_rss_bytes();
    let first_started = Instant::now();
    let first = service.first_page()?;
    let first_page_ms = first_started.elapsed().as_secs_f64() * 1000.0;
    let full_started = Instant::now();
    let full = service.full_index()?;
    let full_index_ms = full_started.elapsed().as_secs_f64() * 1000.0;
    let search_started = Instant::now();
    let search = service.search("src", 200)?;
    let search_ms = search_started.elapsed().as_secs_f64() * 1000.0;
    let encoded_bytes = serde_json::to_vec(&full)?.len();
    let peak_rss_bytes = peak_rss_bytes();
    let peak_additional_rss_bytes = peak_rss_bytes
        .zip(baseline_peak_rss_bytes)
        .map(|(peak, baseline)| peak.saturating_sub(baseline));
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "project": root,
            "model_version": full.model_version,
            "first_page_ms": first_page_ms,
            "full_index_ms": full_index_ms,
            "search_ms": search_ms,
            "first_page_nodes": first.snapshot.nodes.len(),
            "indexed_files": full.indexed_files,
            "indexed_nodes": full.nodes.len(),
            "snapshot_bytes": encoded_bytes,
            "peak_rss_bytes": peak_rss_bytes,
            "peak_additional_rss_bytes": peak_additional_rss_bytes,
            "search_results": search.snapshot.nodes.len().saturating_sub(1),
        }))?
    );
    Ok(())
}
