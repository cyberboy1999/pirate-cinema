//! Local embedded-engine health check; never contacts an external service.
#[cfg(feature = "torrent-engine")]
fn main() -> Result<(), String> {
    let (version, library) = pirate_cinema_core::desktop_engine::read_library("")?;
    println!("librqbit: {version}; library entries: {}", library.len());
    pirate_cinema_core::desktop_engine::shutdown();
    Ok(())
}
#[cfg(not(feature = "torrent-engine"))]
fn main() {
    eprintln!("Enable torrent-engine for this local diagnostic.");
}
