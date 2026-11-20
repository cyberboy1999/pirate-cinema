use pirate_cinema_core::{desktop_engine as engine, preflight};
use std::path::PathBuf;
fn main() -> Result<(), String> {
    let root = PathBuf::from(
        std::env::var_os("PIRATE_CINEMA_DATA_DIR").ok_or("Set isolated PIRATE_CINEMA_DATA_DIR")?,
    );
    let fixture = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Pass owned video fixture directory")?,
    );
    if root.join("rqbit/session/session.json").exists() {
        return Err("Use a fresh isolated PIRATE_CINEMA_DATA_DIR for this smoke test".into());
    }
    engine::read_library("")?;
    let torrent = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            let spawner = librqbit::spawn_utils::BlockingSpawner::new(1);
            librqbit::create_torrent(&fixture, Default::default(), &spawner).await
        })
        .map_err(|e| e.to_string())?;
    let bytes = torrent.as_bytes().map_err(|e| e.to_string())?;
    let hash = librqbit::torrent_from_bytes(&bytes)
        .map_err(|e| e.to_string())?
        .info_hash
        .as_string();
    let downloads = root.join("rqbit/downloads").join(&hash);
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(&fixture).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            std::fs::copy(entry.path(), downloads.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
    }
    let path = root.join("fixture.torrent");
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    engine::import_torrent_file(&path, "Owned preflight fixture")?;
    let files = engine::torrent_video_files("", &hash)?;
    let file = files.first().ok_or("No video")?;
    let db = rusqlite::Connection::open(root.join("history.sqlite")).map_err(|e| e.to_string())?;
    let before: i64 = db
        .query_row("SELECT count(*) FROM media_file_history", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let cancelled = preflight::Check::default();
    cancelled.cancel();
    assert!(preflight::run(&hash, file, &cancelled)
        .unwrap_err()
        .contains("отменена"));
    let report = preflight::run(&hash, file, &preflight::Check::default())?;
    assert!(report.contains("Готова к просмотру"));
    assert!(!engine::is_active(&hash)?);
    let after: i64 = db
        .query_row("SELECT count(*) FROM media_file_history", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    assert_eq!(before, after, "Preflight must not create playback history");
    println!("{report}\nPASS: cancelled check, torrent -> FFmpeg sample, paused swarm, unchanged history");
    engine::shutdown();
    Ok(())
}
