use std::path::PathBuf;
use std::time::Instant;
use vibehub_core::v3::V3ViewRepository;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let repository = V3ViewRepository::open(&root)?;
    let task_id = repository.current_task_id()?;
    let started = Instant::now();
    let bundle = repository.load_bundle(&task_id)?;
    let encoded = serde_json::to_vec(&bundle)?;

    println!(
        "project={} task={} elapsed_ms={:.1} response_bytes={}",
        root.display(),
        task_id,
        started.elapsed().as_secs_f64() * 1000.0,
        encoded.len()
    );
    Ok(())
}
