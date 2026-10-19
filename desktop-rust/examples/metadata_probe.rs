//! Explicit public-title diagnostic; never reads the user's library or history.
fn main() -> Result<(), String> {
    let title = std::env::args()
        .nth(1)
        .ok_or("Pass a public movie/series title")?;
    let series = std::env::args().any(|argument| argument == "--series");
    let start = std::time::Instant::now();
    match pirate_cinema_core::metadata::lookup(&title, series)? {
        Some(movie) => {
            println!(
                "id={} title={} year={:?} overview={} poster={} elapsed={:?}",
                movie.id,
                movie.title,
                movie.year,
                movie.overview.as_ref().map_or(0, String::len),
                movie.poster_url.is_some(),
                start.elapsed()
            );
            if std::env::args().any(|argument| argument == "--poster") {
                println!(
                    "poster_jpeg_bytes={}",
                    pirate_cinema_core::metadata::movie_poster_jpeg(&movie, series)?.len()
                );
            }
        }
        None => println!("No match; elapsed={:?}", start.elapsed()),
    }
    Ok(())
}
