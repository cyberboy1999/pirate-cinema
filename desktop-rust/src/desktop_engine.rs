//! Synchronous desktop boundary around the shared asynchronous engine.
//! All calls are made from blocking workers; the engine owns a separate runtime.
use crate::{embedded::Engine, magnet_info_hash, AddResult, SearchResult, Torrent, VideoFile};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static ENGINE: Mutex<Option<Arc<Engine>>> = Mutex::new(None);
static STREAMS: std::sync::LazyLock<Mutex<std::collections::HashMap<String, usize>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Keeps a chosen swarm live while the local ArtPlayer HLS bridge needs it.
pub struct StreamLease(String);
impl StreamLease {
    pub fn new(hash: &str) -> Result<Self, String> {
        let hash = crate::normalize_info_hash(hash).ok_or("Некорректная раздача")?;
        *STREAMS
            .lock()
            .map_err(|_| "Состояние потоков повреждено")?
            .entry(hash.clone())
            .or_default() += 1;
        Ok(Self(hash))
    }
}
impl Drop for StreamLease {
    fn drop(&mut self) {
        let hash = self.0.clone();
        let cleanup_hash = hash.clone();
        // Drop can run in the UI runtime. Complete pause on a dedicated OS thread.
        let thread = std::thread::spawn(move || {
            let Ok(mut streams) = STREAMS.lock() else {
                return;
            };
            let Some(count) = streams.get_mut(&hash) else {
                return;
            };
            *count = count.saturating_sub(1);
            if *count != 0 {
                return;
            }
            streams.remove(&hash);
            // Keep the lease lock until discarded, so a new lease cannot race late cleanup.
            let active = ENGINE.lock().ok().and_then(|engine| engine.clone());
            if let (Some(engine), Some(runtime)) = (active, RUNTIME.get()) {
                if let Err(error) = runtime.block_on(engine.clear_payload(&hash)) {
                    eprintln!("Не удалось очистить буфер раздачи {hash}: {error}");
                }
            }
        });
        if thread.join().is_err() {
            eprintln!("Не удалось завершить очистку буфера раздачи {cleanup_hash}");
        }
    }
}

fn catalog() -> Result<rusqlite::Connection, String> {
    catalog_at(&profile_dir()?.join("history.sqlite"))
}

fn catalog_at(path: &Path) -> Result<rusqlite::Connection, String> {
    crate::history::HistoryStore::open(path).map_err(|e| e.to_string())?;
    let mut db = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
    db.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    let transaction = db.transaction().map_err(|e| e.to_string())?;
    let exists: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='engine_library')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !exists {
        // Import cached card identities once, without contacting the former service.
        transaction.execute_batch("CREATE TABLE engine_library(hash TEXT PRIMARY KEY,title TEXT NOT NULL);
            INSERT OR IGNORE INTO engine_library SELECT torrent_hash,title FROM media_metadata;
            INSERT OR IGNORE INTO engine_library SELECT torrent_hash,MAX(file_name) FROM media_file_history GROUP BY torrent_hash;")
            .map_err(|e| e.to_string())?;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(db)
}

pub(crate) fn runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime);
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let _ = RUNTIME.set(runtime);
    RUNTIME.get().ok_or("Runtime librqbit недоступен".into())
}

pub fn profile_dir() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("PIRATE_CINEMA_DATA_DIR") {
        return Ok(root.into());
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        let portable = dir.join("portable-data");
        if portable.is_dir() {
            return Ok(portable);
        }
    }
    #[cfg(windows)]
    let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA не задан")?);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .ok_or("HOME не задан")?;
    Ok(root.join("Pirate Cinema"))
}

// Kept as a data-directory helper for existing history/cache callers.
pub fn default_data_dir() -> Result<PathBuf, String> {
    Ok(profile_dir()?.join("rqbit"))
}

fn engine() -> Result<Arc<Engine>, String> {
    let mut engine = ENGINE.lock().map_err(|_| "Состояние librqbit повреждено")?;
    if let Some(engine) = engine.as_ref() {
        return Ok(engine.clone());
    }
    let started = runtime()?.block_on(Engine::start(&profile_dir()?))?;
    *engine = Some(started.clone());
    Ok(started)
}

pub fn health_version(_: &str) -> Result<String, String> {
    if engine()?.healthy() {
        Ok(librqbit::version().into())
    } else {
        Err("Локальный поток librqbit остановлен".into())
    }
}
pub fn read_library(endpoint: &str) -> Result<(String, Vec<Torrent>), String> {
    let version = health_version(endpoint)?;
    let db = catalog()?;
    for torrent in engine()?.list() {
        db.execute(
            "INSERT OR IGNORE INTO engine_library(hash,title) VALUES(?1,?2)",
            (&torrent.hash, &torrent.title),
        )
        .map_err(|e| e.to_string())?;
    }
    let mut query = db
        .prepare("SELECT hash,title FROM engine_library ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;
    let torrents = query
        .query_map([], |row| {
            Ok(Torrent {
                hash: row.get(0)?,
                title: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok((version, torrents))
}
pub fn torrent_video_files(_: &str, hash: &str) -> Result<Vec<VideoFile>, String> {
    let engine = engine()?;
    if !engine.list().iter().any(|item| item.hash == hash) {
        let hash = crate::normalize_info_hash(hash).ok_or("Некорректная раздача")?;
        runtime()?.block_on(engine.add(&format!("magnet:?xt=urn:btih:{hash}")))?;
    }
    engine.files(hash)
}
pub fn stream_url(_: &str, hash: &str, file: &VideoFile) -> Result<String, String> {
    runtime()?.block_on(engine()?.play_url(hash, file))
}
pub fn is_active(hash: &str) -> Result<bool, String> {
    engine()?.is_active(hash)
}
pub fn add_magnet(_: &str, magnet: &str, title: &str) -> Result<AddResult, String> {
    if magnet.starts_with("http://") || magnet.starts_with("https://") {
        let preferences = crate::settings::load_preferences(&profile_dir()?.join("settings.json"))?;
        let origin = |url: &str| url.split('/').take(3).collect::<Vec<_>>().join("/");
        if preferences.torznab_url.is_empty() || origin(magnet) != origin(&preferences.torznab_url)
        {
            return Err("Ссылка загрузки не принадлежит настроенному Torznab".into());
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(20)))
            .build()
            .into();
        let mut response = agent
            .get(magnet)
            .call()
            .map_err(|_| "Torznab: не удалось скачать torrent")?;
        let bytes = response
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_vec()
            .map_err(|_| "Torznab: ошибка чтения torrent")?;
        let hash = librqbit::torrent_from_bytes(&bytes)
            .map_err(|_| "Torznab вернул некорректный torrent")?
            .info_hash
            .as_string();
        let engine = engine()?;
        let already_exists = engine.list().iter().any(|item| item.hash == hash);
        runtime()?.block_on(engine.add_torrent_file(bytes))?;
        catalog()?.execute("INSERT INTO engine_library(hash,title) VALUES(?1,?2) ON CONFLICT(hash) DO UPDATE SET title=excluded.title", (&hash, title)).map_err(|e| e.to_string())?;
        return Ok(AddResult {
            hash,
            already_exists,
        });
    }
    let hash = magnet_info_hash(magnet).ok_or("Нужна корректная magnet-ссылка")?;
    let engine = engine()?;
    let already_exists = engine.list().iter().any(|item| item.hash == hash);
    runtime()?.block_on(engine.add(magnet))?;
    catalog()?.execute("INSERT INTO engine_library(hash,title) VALUES(?1,?2) ON CONFLICT(hash) DO UPDATE SET title=excluded.title", (&hash, title)).map_err(|e| e.to_string())?;
    Ok(AddResult {
        hash,
        already_exists,
    })
}

pub fn import_torrent_file(path: &Path, title: &str) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let hash = runtime()?.block_on(engine()?.add_torrent_file(bytes))?;
    catalog()?
        .execute(
            "INSERT OR IGNORE INTO engine_library(hash,title) VALUES(?1,?2)",
            (&hash, title),
        )
        .map_err(|e| e.to_string())?;
    Ok(hash)
}
pub fn remove_torrent(_: &str, hash: &str) -> Result<(), String> {
    let engine = engine()?;
    if engine.list().iter().any(|item| item.hash == hash) {
        runtime()?.block_on(engine.remove(hash))?;
    }
    catalog()?
        .execute("DELETE FROM engine_library WHERE hash=?1", [hash])
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn search_all_sources(
    _: &str,
    selected_source: &str,
    query: &str,
) -> Result<Vec<SearchResult>, String> {
    let root = profile_dir()?;
    let path = root.join("settings.json");
    let source = if selected_source.is_empty() {
        crate::settings::search_source(&path)?
    } else {
        selected_source.to_owned()
    };
    let preferences = crate::settings::load_preferences(&path)?;
    let mut queries = vec![query.trim().to_owned()];
    let movie = crate::metadata::lookup(query, false)
        .ok()
        .flatten()
        .or_else(|| crate::metadata::lookup(query, true).ok().flatten());
    if let Some(movie) = movie {
        for title in [movie.title, movie.original_title] {
            if !title.trim().is_empty() && !queries.iter().any(|q| q.eq_ignore_ascii_case(&title)) {
                queries.push(title);
            }
        }
    }
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let mut succeeded = false;
    for query in queries.iter().take(3) {
        let response = match source.as_str() {
            "torrents_csv" => search_torrents_csv(query),
            "torznab" => search_torznab(
                &preferences.torznab_url,
                &preferences.torznab_api_key,
                query,
            ),
            _ => crate::embedded::search(&root, query),
        };
        match response {
            Ok(items) => {
                succeeded = true;
                found.extend(items);
            }
            Err(error) => errors.push(error),
        }
    }
    if !succeeded {
        return Err(errors.join("; "));
    }
    let wanted_year = crate::catalog::clean_title(query).1;
    let mut seen = std::collections::HashSet::new();
    found.retain(|item| {
        let year = crate::catalog::clean_title(&item.title).1;
        !wanted_year.is_some_and(|wanted| year.is_some_and(|actual| actual != wanted))
            && seen.insert(item.hash.clone())
    });
    Ok(found)
}
pub fn shutdown() {
    // Kill/wait only children created here before stopping the owned torrent session.
    if let Ok(mut state) = ENGINE.lock() {
        if let Some(engine) = state.take() {
            if let Some(runtime) = RUNTIME.get() {
                let _ = std::thread::spawn(move || runtime.block_on(engine.stop())).join();
            }
        }
    }
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn relevant_results(results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
    let words = |text: &str| {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let wanted = words(query);
    let mut seen = std::collections::HashSet::new();
    results
        .into_iter()
        .filter(|item| {
            let title = words(&item.title);
            wanted.iter().all(|word| title.contains(word))
                && !title.iter().any(|word| {
                    [
                        "mp3",
                        "flac",
                        "repack",
                        "fitgirl",
                        "igruha",
                        "soundtrack",
                        "ost",
                    ]
                    .contains(&word.as_str())
                })
                && seen.insert(item.hash.clone())
        })
        .collect()
}
fn search_torrents_csv(query: &str) -> Result<Vec<SearchResult>, String> {
    if query.trim().chars().count() < 3 {
        return Err("Torrents.csv: минимум 3 символа".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent
        .get(format!(
            "https://torrents-csv.com/service/search?size=100&q={}",
            encode(query.trim())
        ))
        .call()
        .map_err(|_| "Torrents.csv: сервис недоступен")?;
    let body = response
        .body_mut()
        .with_config()
        .limit(4 * 1024 * 1024)
        .read_to_string()
        .map_err(|_| "Torrents.csv: не удалось прочитать ответ")?;
    let mut found = parse_torrents_csv(&body)?;
    let mut next = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("next").and_then(|n| n.as_u64()));
    for _ in 0..3 {
        let Some(cursor) = next else {
            break;
        };
        let mut response = agent
            .get(format!(
                "https://torrents-csv.com/service/search?q={}&after={cursor}",
                encode(query.trim())
            ))
            .call()
            .map_err(|_| "Torrents.csv: ошибка следующей страницы")?;
        let body = response
            .body_mut()
            .with_config()
            .limit(4 * 1024 * 1024)
            .read_to_string()
            .map_err(|_| "Torrents.csv: ошибка чтения страницы")?;
        for item in parse_torrents_csv(&body)? {
            if !found.iter().any(|saved| saved.hash == item.hash) {
                found.push(item);
            }
        }
        let following = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("next").and_then(|n| n.as_u64()));
        if following == next || found.len() >= 100 {
            break;
        }
        next = following;
    }
    Ok(relevant_results(found, query))
}

fn parse_torrents_csv(body: &str) -> Result<Vec<SearchResult>, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| "Torrents.csv: ожидался JSON, возможно включена защита сайта")?;
    let torrents = value
        .get("torrents")
        .and_then(|v| v.as_array())
        .ok_or("Torrents.csv: неверный формат ответа")?;
    Ok(torrents
        .iter()
        .filter_map(|item| {
            let hash = item.get("infohash")?.as_str()?.to_ascii_lowercase();
            let title = item.get("name")?.as_str()?.trim();
            if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) || title.is_empty()
            {
                return None;
            }
            Some(SearchResult {
                magnet: format!("magnet:?xt=urn:btih:{hash}&dn={}", encode(title)),
                hash,
                title: title.to_owned(),
                size: format!(
                    "{:.2} GiB",
                    item.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0) as f64
                        / 1073741824.0
                ),
                seeders: item
                    .get("seeders")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .min(i64::MAX as u64) as i64,
                source: "Torrents.csv".into(),
                tracker: String::new(),
            })
        })
        .take(100)
        .collect())
}
fn video_categories(xml: &str) -> Result<Vec<String>, String> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    let mut categories = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|_| "Torznab: некорректный t=caps")?
        {
            Event::Start(tag) | Event::Empty(tag)
                if matches!(tag.local_name().as_ref(), b"category" | b"subcat") =>
            {
                for attribute in tag.attributes().flatten() {
                    if attribute.key.as_ref() == b"id" {
                        let id = attribute
                            .unescape_value()
                            .map_err(|_| "Torznab: неверная категория")?;
                        if let Ok(number) = id.parse::<u32>() {
                            if ((2000..3000).contains(&number)
                                || (5000..6000).contains(&number)
                                || (8000..8100).contains(&number))
                                && !categories.iter().any(|saved| saved == id.as_ref())
                            {
                                categories.push(id.into_owned());
                            }
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if categories.is_empty() {
        return Err("Torznab: индексер не сообщил категории видео".into());
    }
    Ok(categories)
}
fn search_torznab(host: &str, key: &str, query: &str) -> Result<Vec<SearchResult>, String> {
    crate::settings::validate_torznab(host, key)?;
    let host = host.trim();
    let endpoint = if host.to_ascii_lowercase().contains("/api/v2.0/indexers/")
        && !host.trim_end_matches('/').ends_with("/api")
    {
        format!("{}/api", host.trim_end_matches('/'))
    } else {
        host.to_owned()
    };
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(15)))
        .build()
        .into();
    let caps_url = format!("{endpoint}{separator}t=caps&apikey={}", encode(key.trim()));
    let mut caps_response = agent
        .get(caps_url)
        .call()
        .map_err(|_| "Torznab: не удалось получить категории t=caps")?;
    let caps = caps_response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|_| "Torznab: ошибка чтения t=caps")?;
    let categories = video_categories(&caps)?.join(",");
    let mut response = agent
        .get(format!(
            "{endpoint}{separator}t=search&cat={categories}&limit=100&apikey={}&q={}",
            encode(key.trim()),
            encode(query)
        ))
        .call()
        .map_err(|_| "Jackett/Torznab: ошибка запроса (адрес и ключ скрыты)")?;
    use std::io::Read;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut body)
        .map_err(|_| "Jackett/Torznab: ошибка чтения")?;
    if body.len() > 4 * 1024 * 1024 {
        return Err("Jackett/Torznab: слишком большой ответ".into());
    }
    Ok(relevant_results(parse_torznab(&body)?, query))
}

fn parse_torznab(body: &str) -> Result<Vec<SearchResult>, String> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);
    let mut found = Vec::new();
    let mut item: Option<SearchResult> = None;
    let mut field = Vec::new();
    let mut rss = false;
    loop {
        match reader
            .read_event()
            .map_err(|_| "Некорректный XML Jackett/Torznab")?
        {
            Event::Start(tag) => {
                let name = tag.local_name();
                if name.as_ref() == b"rss" {
                    rss = true;
                }
                if name.as_ref() == b"item" {
                    item = Some(SearchResult {
                        title: String::new(),
                        magnet: String::new(),
                        hash: String::new(),
                        size: String::new(),
                        seeders: 0,
                        source: "Jackett/Torznab".into(),
                        tracker: String::new(),
                    });
                }
                field = name.as_ref().to_vec();
            }
            Event::Empty(tag) if item.is_some() => {
                let mut name = String::new();
                let mut value = String::new();
                let mut enclosure_size = None;
                for attribute in tag.attributes() {
                    let attribute = attribute.map_err(|_| "Некорректные атрибуты Torznab")?;
                    let text = attribute
                        .unescape_value()
                        .map_err(|_| "Некорректный XML Torznab")?
                        .into_owned();
                    match attribute.key.as_ref() {
                        b"name" => name = text,
                        b"value" | b"url" => value = text,
                        b"length" => enclosure_size = text.parse::<u64>().ok(),
                        _ => {}
                    }
                }
                let item = item.as_mut().unwrap();
                if item.size.is_empty() {
                    if let Some(bytes) = enclosure_size {
                        item.size = bytes.to_string();
                    }
                }
                match name.as_str() {
                    "magneturl" => item.magnet = value,
                    "infohash" => item.hash = value,
                    "seeders" => item.seeders = value.parse::<i64>().unwrap_or(0).max(0),
                    "size" => item.size = value,
                    _ if tag.local_name().as_ref() == b"enclosure"
                        && (value.starts_with("magnet:?")
                            || value.starts_with("http://")
                            || value.starts_with("https://")) =>
                    {
                        item.magnet = value
                    }
                    _ => {}
                }
            }
            Event::Text(text) if item.is_some() => {
                let text = text.unescape().map_err(|_| "Некорректный текст Torznab")?;
                let item = item.as_mut().unwrap();
                match field.as_slice() {
                    b"title" => item.title.push_str(&text),
                    b"link"
                        if text.starts_with("magnet:?")
                            || text.starts_with("http://")
                            || text.starts_with("https://") =>
                    {
                        item.magnet = text.into_owned()
                    }
                    b"size" => item.size = text.into_owned(),
                    _ => {}
                }
            }
            Event::End(tag) => {
                if tag.local_name().as_ref() == b"item" {
                    if let Some(mut item) = item.take() {
                        if let Ok(bytes) = item.size.trim().parse::<u64>() {
                            item.size = if bytes >= 1073741824 {
                                format!("{:.2} GiB", bytes as f64 / 1073741824.0)
                            } else {
                                format!("{:.1} MiB", bytes as f64 / 1048576.0)
                            };
                        }
                        if let Some(hash) = magnet_info_hash(&item.magnet)
                            .or_else(|| crate::normalize_info_hash(&item.hash))
                        {
                            item.hash = hash.clone();
                            if item.magnet.is_empty() {
                                item.magnet = format!("magnet:?xt=urn:btih:{hash}");
                            }
                            if !found.iter().any(|saved: &SearchResult| saved.hash == hash) {
                                found.push(item);
                            }
                        } else if item.magnet.starts_with("http://")
                            || item.magnet.starts_with("https://")
                        {
                            use std::hash::{Hash, Hasher};
                            let mut id = std::collections::hash_map::DefaultHasher::new();
                            item.magnet.hash(&mut id);
                            item.hash = format!("torznab:{:016x}", id.finish());
                            if !found
                                .iter()
                                .any(|saved: &SearchResult| saved.hash == item.hash)
                            {
                                found.push(item);
                            }
                        }
                    }
                }
                field.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !rss {
        return Err("Jackett/Torznab вернул ответ без RSS".into());
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_card_import_is_once_and_forgetting_never_reimports_history() {
        let root = std::env::temp_dir().join(format!("pc-catalog-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("history.sqlite");
        let history = crate::history::HistoryStore::open(&path).unwrap();
        let hash = "a".repeat(40);
        history.set_title(&hash, "Cached title").unwrap();
        history.mark_played(&hash, 1, "Episode.mkv", None).unwrap();
        let db = catalog_at(&path).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM engine_library", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        db.execute("DELETE FROM engine_library", []).unwrap();
        drop(db);
        let db = catalog_at(&path).unwrap();
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM engine_library", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(history.get(&hash, 1).unwrap().is_some());
        drop(db);
        drop(history);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn direct_torznab_preserves_magnets_and_rejects_login_pages() {
        let hash = "a".repeat(40);
        let xml = format!(
            r#"<rss xmlns:torznab="urn:test"><channel><item><title>Film &amp; Series</title><torznab:attr name="magneturl" value="magnet:?xt=urn:btih:{hash}&amp;tr=https%3A%2F%2Ftracker.test"/><torznab:attr name="seeders" value="12"/></item></channel></rss>"#
        );
        let result = parse_torznab(&xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Film & Series");
        assert_eq!(result[0].seeders, 12);
        assert!(result[0].magnet.contains("&tr="));
        assert!(parse_torznab("<html>Login</html>").is_err());
        assert_eq!(encode("Тачки &"), "%D0%A2%D0%B0%D1%87%D0%BA%D0%B8%20%26");
    }
    #[test]
    fn caps_keep_video_animation_and_unknown_but_exclude_games_audio() {
        let xml = r#"<caps><categories><category id="2000"><subcat id="2040"/></category><category id="5000"><subcat id="5070"/></category><category id="8010"/><category id="4000"/><category id="3000"/></categories></caps>"#;
        assert_eq!(
            video_categories(xml).unwrap(),
            ["2000", "2040", "5000", "5070", "8010"]
        );
        assert!(video_categories("<html>Login</html>").is_err());
    }

    #[test]
    fn relevance_rejects_substring_games_and_audio_and_deduplicates() {
        let rows = [
            "Cars (2006)",
            "My Summer Car",
            "Cars FitGirl Repack",
            "Cars soundtrack mp3",
            "Cars (2006)",
        ];
        let items = rows
            .iter()
            .map(|title| SearchResult {
                title: (*title).into(),
                hash: (*title).into(),
                magnet: String::new(),
                size: String::new(),
                seeders: 1,
                source: String::new(),
                tracker: String::new(),
            })
            .collect();
        let found = relevant_results(items, "Cars");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Cars (2006)");
    }
    #[test]
    fn torznab_keeps_download_links_without_infohash() {
        let xml = r#"<rss><channel><item><title>Тачки (2006)</title><link>http://127.0.0.1:9117/dl/test</link><enclosure url="http://127.0.0.1:9117/dl/test" type="application/x-bittorrent"/></item></channel></rss>"#;
        let found = parse_torznab(xml).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Тачки (2006)");
        assert!(found[0].hash.starts_with("torznab:"));
        assert!(found[0].magnet.starts_with("http://"));
    }
    #[test]
    fn torznab_formats_byte_sizes_and_reads_enclosure_length() {
        let xml = r#"<rss><channel><item><title>Film</title><enclosure url="http://127.0.0.1/dl" length="2147483648"/></item><item><title>Other</title><size>104857600</size><link>http://127.0.0.1/other</link></item></channel></rss>"#;
        let found = parse_torznab(xml).unwrap();
        assert_eq!(found[0].size, "2.00 GiB");
        assert_eq!(found[1].size, "100.0 MiB");
    }
}
