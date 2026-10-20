//! Local diagnostic. Explicit public query only; never reads the user's library.
#[allow(dead_code)]
#[path = "../src/embedded.rs"]
mod embedded;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let profile = std::path::PathBuf::from(args.next().ok_or("Укажите отдельную папку теста")?);
    let query = args.next().ok_or("Укажите публичное название для поиска")?;
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        let engine = embedded::Engine::start(&profile).await?;
        println!(
            "librqbit {}: session ready, saved torrents: {}",
            librqbit::version(),
            engine.list().len()
        );
        let results = tokio::task::spawn_blocking(move || embedded::search(&profile, &query))
            .await
            .map_err(|e| e.to_string())??;
        println!("Found: {}", results.len());
        for item in results.iter().take(8) {
            println!("{} | seeds={} | {}", item.title, item.seeders, item.size);
        }
        if results.is_empty() {
            return Err("Нет результатов".into());
        }
        Ok(())
    })
}
