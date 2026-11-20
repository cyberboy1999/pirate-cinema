use pirate_cinema_core::{desktop_engine as engine, preflight};
fn main() -> Result<(), String> {
    let torrent = std::path::PathBuf::from(std::env::args().nth(1).ok_or("torrent required")?);
    let hash = engine::import_torrent_file(&torrent, "Explicit VPN diagnostic")?;
    let files = engine::torrent_video_files("", &hash)?;
    let file = files
        .iter()
        .find(|f| f.id == 19)
        .or_else(|| files.first())
        .ok_or("No video")?;
    let worker_hash = hash.clone();
    let worker_file = file.clone();
    let worker = std::thread::spawn(move || {
        preflight::run(&worker_hash, &worker_file, &preflight::Check::default())
    });
    while !worker.is_finished() {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let stats = engine::torrent_diagnostics(&hash)?;
        println!(
            "live state={} bytes={} peers={} dht={}",
            stats["state"],
            stats["progress_bytes"],
            stats["live"]["snapshot"]["peer_stats"],
            stats["dht"]
        );
    }
    let result = worker.join().map_err(|_| "Diagnostic panicked")?;
    println!("result: {result:?}");
    println!("stats after check: {}", engine::torrent_diagnostics(&hash)?);
    engine::shutdown();
    result.map(|_| ())
}
