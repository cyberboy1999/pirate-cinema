use pirate_cinema_core::{
    history::{HistoryStore, MediaMetadata},
    metadata, normalize_info_hash,
};
use std::path::Path;

pub fn recent_cards(profile: &Path) -> Result<Vec<pirate_cinema_core::Torrent>, String> {
    let db = store(profile)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut cards = Vec::new();
    for entry in db.recently_played(200).map_err(|e| e.to_string())? {
        if seen.insert(entry.torrent_hash.clone()) {
            let title = db
                .metadata(&entry.torrent_hash)
                .map_err(|e| e.to_string())?
                .map_or(entry.file_name, |value| value.title);
            cards.push(pirate_cinema_core::Torrent {
                hash: entry.torrent_hash,
                title,
            });
        }
    }
    Ok(cards)
}

pub fn store(profile: &Path) -> Result<HistoryStore, String> {
    std::fs::create_dir_all(profile).map_err(|e| e.to_string())?;
    HistoryStore::open(&profile.join("history.sqlite")).map_err(|e| e.to_string())
}

/// Explicit enrichment only. Empty/error responses never erase existing metadata.
pub fn refresh(
    profile: &Path,
    hash: &str,
    title: &str,
    series: bool,
) -> Result<(MediaMetadata, Option<String>), String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректная раздача")?;
    let movie = metadata::lookup(title, series)?
        .ok_or("Совпадений не найдено. Прежние описание и постер сохранены.")?;
    let (poster, warning) = match metadata::movie_poster_jpeg(&movie, series) {
        Ok(bytes) => (Some(bytes), None),
        Err(error) => (
            None,
            Some(format!("Описание сохранено, постер не обновлён: {error}")),
        ),
    };
    save(profile, &hash, title, &movie, poster.as_deref())?;
    let saved = store(profile)?
        .metadata(&hash)
        .map_err(|e| e.to_string())?
        .ok_or("Метаданные не сохранились")?;
    Ok((saved, warning))
}

fn save(
    profile: &Path,
    hash: &str,
    title: &str,
    movie: &metadata::Movie,
    poster: Option<&[u8]>,
) -> Result<(), String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректная раздача")?;
    let poster_file = if let Some(bytes) = poster {
        if bytes.len() > 5 * 1024 * 1024 || !bytes.starts_with(&[0xff, 0xd8]) {
            return Err("Некорректный JPEG-постер".into());
        }
        let directory = profile.join("posters");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let name = format!("{hash}.jpg");
        let temporary = directory.join(format!("{hash}.jpg.part"));
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temporary, directory.join(&name)).map_err(|e| e.to_string())?;
        Some(name)
    } else {
        None
    };
    let value = MediaMetadata {
        title: movie.title.clone(),
        overview: movie.overview.clone(),
        year: movie.year,
        rating: movie.rating,
        poster_file,
        genres: movie.genres.clone(),
    };
    let db = store(profile)?;
    // This screen edits the lookup query, not a separate user-defined display title.
    let previous_title = db
        .metadata(&hash)
        .map_err(|e| e.to_string())?
        .map(|entry| entry.title)
        .unwrap_or_else(|| title.to_owned());
    db.merge_remote_metadata(&hash, &previous_title, &value)
        .map_err(|e| e.to_string())
}

/// The WebView can request only a canonical hash JPEG, never arbitrary profile files.
pub fn poster_bytes(profile: &Path, request_path: &str) -> Option<Vec<u8>> {
    let hash = request_path
        .strip_prefix("/posters/")?
        .strip_suffix(".jpg")?;
    if normalize_info_hash(hash).as_deref() != Some(hash) {
        return None;
    }
    let path = profile.join("posters").join(format!("{hash}.jpg"));
    if path.metadata().ok()?.len() > 5 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    bytes.starts_with(&[0xff, 0xd8]).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_cards_work_without_a_server_and_deduplicate_episodes() {
        let root = std::env::temp_dir().join(format!("pc-mobile-offline-{}", std::process::id()));
        let db = store(&root).unwrap();
        let hash = "c".repeat(40);
        db.mark_played(&hash, 1, "Episode one", None).unwrap();
        db.mark_played(&hash, 2, "Episode two", None).unwrap();
        let cards = recent_cards(&root).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].hash, hash);
        drop(db);
        std::fs::remove_file(root.join("history.sqlite")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn metadata_and_poster_survive_reopen_and_preserve_russian_text() {
        let root = std::env::temp_dir().join(format!("pc-mobile-metadata-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let hash = "b".repeat(40);
        let mut movie = metadata::Movie {
            id: "tt123".into(),
            title: "Фильм".into(),
            original_title: "Film".into(),
            year: Some(2020),
            rating: None,
            poster_url: None,
            alternate_poster_url: None,
            background_url: None,
            overview: Some("Русское описание".into()),
            genres: vec![],
        };
        let jpeg = &[0xff, 0xd8, 0xff, 0xd9];
        save(&root, &hash, "Release", &movie, Some(jpeg)).unwrap();
        movie.overview = Some("English fallback".into());
        movie.title = "Уточнённое название".into();
        save(&root, &hash, "Release", &movie, None).unwrap();
        let saved = store(&root).unwrap().metadata(&hash).unwrap().unwrap();
        assert_eq!(saved.title, "Уточнённое название");
        assert_eq!(saved.overview.as_deref(), Some("Русское описание"));
        assert_eq!(
            poster_bytes(&root, &format!("/posters/{hash}.jpg")).as_deref(),
            Some(jpeg.as_slice())
        );
        assert!(poster_bytes(&root, "/posters/../history.sqlite.jpg").is_none());
        assert!(poster_bytes(&root, "/posters/invalid.jpg").is_none());
        std::fs::remove_file(root.join("posters").join(format!("{hash}.jpg"))).unwrap();
        std::fs::remove_dir(root.join("posters")).unwrap();
        std::fs::remove_file(root.join("history.sqlite")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
