//! Explicit public-title search check using an isolated profile.
fn main() -> Result<(), String> {
    let query = std::env::args().nth(1).ok_or("Pass a public title")?;
    let source = std::env::args().nth(2).unwrap_or_default();
    let results = pirate_cinema_core::desktop_engine::search_all_sources("", &source, &query)?;
    println!("found={}", results.len());
    for item in results.iter().take(8) {
        println!("{} | seeds={} | {}", item.title, item.seeders, item.size);
    }
    pirate_cinema_core::desktop_engine::shutdown();
    if results.is_empty() {
        return Err("No results".into());
    }
    Ok(())
}
