pub const DEFAULT_TORRSERVER_URL: &str = "http://127.0.0.1:8090";
pub mod catalog;
pub mod history;
pub mod metadata;
pub mod migration;
pub mod mpv;
pub mod settings;
pub mod torrserver_process;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Torrent {
    pub title: String,
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub title: String,
    pub magnet: String,
    pub hash: String,
    pub size: String,
    pub seeders: i64,
    pub source: String,
    pub tracker: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AddResult {
    pub hash: String,
    pub already_exists: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoFile {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseUpdate {
    pub version: String,
    pub download_url: String,
}

pub fn check_rust_update(current: &str) -> Result<Option<ReleaseUpdate>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .user_agent("Pirate-Cinema-Rust")
        .build()
        .into();
    let mut response = agent
        .get("https://api.github.com/repos/cyberboy1999/pirate-cinema/releases?per_page=12")
        .header("accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("GitHub не ответил: {error}"))?;
    let releases: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный ответ GitHub: {error}"))?;
    let update = releases
        .as_array()
        .into_iter()
        .flatten()
        .filter(|release| {
            !release
                .get("draft")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .find_map(|release| {
            let version = release
                .get("tag_name")?
                .as_str()?
                .trim_start_matches(['v', 'V']);
            if !newer_version(version, current) {
                return None;
            }
            let asset = release.get("assets")?.as_array()?.iter().find(|asset| {
                let name = asset
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                name.starts_with("pirate-cinema-setup-") && name.ends_with("-win-x64.exe")
            })?;
            Some(ReleaseUpdate {
                version: version.to_owned(),
                download_url: asset.get("browser_download_url")?.as_str()?.to_owned(),
            })
        });
    Ok(update)
}

pub fn download_rust_update(update: &ReleaseUpdate) -> Result<std::path::PathBuf, String> {
    use std::io::Read as _;
    const MAX_INSTALLER_BYTES: u64 = 300 * 1024 * 1024;
    let expected_prefix = "https://github.com/cyberboy1999/pirate-cinema/releases/download/";
    if !update.download_url.starts_with(expected_prefix) || !update.download_url.ends_with(".exe") {
        return Err("GitHub вернул недопустимый адрес установщика".into());
    }
    let directory = std::env::temp_dir().join("Pirate Cinema Updates");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Не удалось создать папку обновления: {error}"))?;
    let destination = directory.join(format!(
        "Pirate-Cinema-Setup-{}-win-x64.exe",
        update.version
    ));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(180)))
        .user_agent("Pirate-Cinema-Rust")
        .build()
        .into();
    let mut response = agent
        .get(&update.download_url)
        .call()
        .map_err(|error| format!("Не удалось скачать обновление: {error}"))?;
    if response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|size| size > MAX_INSTALLER_BYTES)
    {
        return Err("Установщик обновления слишком большой".into());
    }
    let temporary = destination.with_extension("exe.part");
    let mut file = std::fs::File::create(&temporary)
        .map_err(|error| format!("Не удалось сохранить обновление: {error}"))?;
    let copied = std::io::copy(
        &mut response
            .body_mut()
            .as_reader()
            .take(MAX_INSTALLER_BYTES + 1),
        &mut file,
    )
    .map_err(|error| format!("Загрузка обновления прервана: {error}"))?;
    if copied > MAX_INSTALLER_BYTES {
        let _ = std::fs::remove_file(&temporary);
        return Err("Установщик обновления слишком большой".into());
    }
    let header = std::fs::read(&temporary)
        .map_err(|error| format!("Не удалось проверить обновление: {error}"))?;
    if header.len() < 2 || &header[..2] != b"MZ" {
        let _ = std::fs::remove_file(&temporary);
        return Err("Загруженный файл не является установщиком Windows".into());
    }
    std::fs::rename(&temporary, &destination)
        .map_err(|error| format!("Не удалось подготовить обновление: {error}"))?;
    Ok(destination)
}

fn newer_version(candidate: &str, current: &str) -> bool {
    let numbers = |value: &str| {
        value
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .take(3)
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .chain(std::iter::repeat(0))
            .take(3)
            .collect::<Vec<_>>()
    };
    numbers(candidate) > numbers(current)
}

pub fn torrent_video_files(base_url: &str, hash: &str) -> Result<Vec<VideoFile>, String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректный хеш раздачи")?;
    let base_url = base_url.trim_end_matches('/');
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    // TorrServer already caches file_stats in its saved torrent list. Asking
    // /stream/details first can block for metadata and time out on every card.
    if let Ok(mut list) = agent
        .post(format!("{base_url}/torrents"))
        .send_json(serde_json::json!({"action": "list"}))
    {
        if let Ok(value) = list.body_mut().read_json::<serde_json::Value>() {
            let items = value
                .as_array()
                .or_else(|| value.get("torrents").and_then(|value| value.as_array()))
                .or_else(|| value.get("items").and_then(|value| value.as_array()));
            if let Some(saved) = items.and_then(|items| {
                items.iter().find(|item| {
                    ["hash", "Hash", "info_hash"]
                        .iter()
                        .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                        .and_then(normalize_info_hash)
                        .as_deref()
                        == Some(hash.as_str())
                })
            }) {
                let cached = parse_video_files(saved);
                if !cached.is_empty() {
                    return Ok(cached);
                }
            }
        }
    }
    let url = format!("{base_url}/stream/details?link={hash}&stat");
    let mut response = None;
    for attempt in 0..2 {
        match agent.get(&url).call() {
            Ok(result) => {
                response = Some(result);
                break;
            }
            Err(error) if attempt < 1 && retryable_details_error(&error) => {
                std::thread::sleep(std::time::Duration::from_secs(attempt + 1));
            }
            Err(error) => return Err(format!("Не удалось получить файлы раздачи: {error}")),
        }
    }
    let mut response = response.ok_or("TorrServer не ответил после повторных запросов")?;
    let details: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный список файлов: {error}"))?;
    let videos = parse_video_files(&details);
    if videos.is_empty() {
        return Err("TorrServer ещё не получил список видеофайлов. Повторите позже".into());
    }
    Ok(videos)
}

fn parse_video_files(details: &serde_json::Value) -> Vec<VideoFile> {
    let saved_data = details.get("data").and_then(|data| {
        data.as_str()
            .map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
            .unwrap_or_else(|| Some(data.clone()))
    });
    let Some(files) = details
        .get("file_stats")
        .and_then(|value| value.as_array())
        .or_else(|| details.get("files").and_then(|value| value.as_array()))
        .or_else(|| {
            saved_data
                .as_ref()
                .and_then(|data| data.get("TorrServer"))
                .and_then(|data| data.get("Files"))
                .and_then(|files| files.as_array())
        })
    else {
        return Vec::new();
    };
    let mut videos = files
        .iter()
        .enumerate()
        .filter_map(|(position, file)| {
            let path = file
                .get("path")
                .or_else(|| file.get("name"))?
                .as_str()?
                .to_owned();
            let name = path.rsplit(['/', '\\']).next()?.to_owned();
            let extension = name.rsplit_once('.')?.1;
            if !["mkv", "mp4", "avi", "mov", "m4v", "webm", "ts", "m2ts"]
                .iter()
                .any(|accepted| extension.eq_ignore_ascii_case(accepted))
            {
                return None;
            }
            let id = file
                .get("id")
                .or_else(|| file.get("index"))
                .and_then(|value| value.as_i64())
                .unwrap_or(position as i64 + 1);
            if id < 1 {
                return None;
            }
            let length = file
                .get("length")
                .or_else(|| file.get("size"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            Some(VideoFile {
                id,
                name,
                path,
                length,
            })
        })
        .collect::<Vec<_>>();
    videos.sort_by(|left, right| natural_path_cmp(&left.path, &right.path));
    videos
}

fn natural_path_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let mut a = left.chars().peekable();
    let mut b = right.chars().peekable();
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let digits = |chars: &mut std::iter::Peekable<std::str::Chars<'_>>| {
                    let mut number = String::new();
                    while chars.peek().is_some_and(char::is_ascii_digit) {
                        number.push(chars.next().unwrap());
                    }
                    number
                };
                let x = digits(&mut a);
                let y = digits(&mut b);
                let order = x
                    .trim_start_matches('0')
                    .len()
                    .cmp(&y.trim_start_matches('0').len())
                    .then_with(|| x.trim_start_matches('0').cmp(y.trim_start_matches('0')));
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.cmp(&y);
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
        }
    }
}

fn retryable_details_error(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::Timeout(_) | ureq::Error::StatusCode(400 | 409 | 500) | ureq::Error::Io(_)
    )
}

pub fn stream_url(base_url: &str, hash: &str, file: &VideoFile) -> Result<String, String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректный хеш раздачи")?;
    if file.id < 1 {
        return Err("Некорректный индекс файла".into());
    }
    let encoded: String = file
        .name
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect();
    Ok(format!(
        "{}/stream/{encoded}?link={hash}&index={}&play",
        base_url.trim_end_matches('/'),
        file.id
    ))
}

pub fn probe_stream(base_url: &str, hash: &str, file: &VideoFile) -> Result<(), String> {
    let url = stream_url(base_url, hash, file)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    let mut response = agent
        .get(&url)
        .header("Range", "bytes=0-0")
        .call()
        .map_err(|error| match error {
            ureq::Error::StatusCode(status) => {
                format!("TorrServer вернул HTTP {status}: видеоданные пока недоступны")
            }
            ureq::Error::Timeout(_) => "TorrServer не отдал видеоданные за 10 секунд".into(),
            other => format!("Поток недоступен: {other}"),
        })?;
    use std::io::Read;
    let mut first = [0_u8; 1];
    match response.body_mut().as_reader().read(&mut first) {
        Ok(1) => Ok(()),
        Ok(_) => Err("TorrServer вернул пустой поток".into()),
        Err(error) => Err(format!("Не удалось прочитать видеопоток: {error}")),
    }
}

pub fn search_torrserver(base_url: &str, query: &str) -> Result<Vec<SearchResult>, String> {
    search_torrserver_path(base_url, query, "/search", "RuTor", None)
}

pub fn search_torznab(
    base_url: &str,
    torznab_url: &str,
    query: &str,
) -> Result<Vec<SearchResult>, String> {
    let source = if torznab_url
        .to_ascii_lowercase()
        .contains("/api/v2.0/indexers/")
    {
        "Jackett"
    } else {
        "Torznab"
    };
    search_torrserver_path(
        base_url,
        query,
        "/torznab/search/",
        source,
        Some(torznab_url),
    )
}

pub fn read_torznab_config(base_url: &str) -> Result<Option<(String, String)>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    let mut response = agent
        .post(format!("{}/settings", base_url.trim_end_matches('/')))
        .send_json(serde_json::json!({"action":"get"}))
        .map_err(|error| format!("Не удалось прочитать настройки TorrServer: {error}"))?;
    let sets: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректные настройки TorrServer: {error}"))?;
    if !sets
        .get("EnableTorznabSearch")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    Ok(sets
        .get("TorznabUrls")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                let host = item.get("Host")?.as_str()?.trim();
                let key = item.get("Key")?.as_str()?.trim();
                (!host.is_empty() && !key.is_empty()).then(|| (host.to_owned(), key.to_owned()))
            })
        }))
}

pub fn configure_torznab(base_url: &str, host: &str, key: &str) -> Result<(), String> {
    crate::settings::validate_torznab(host, key)?;
    if !host.trim().is_empty() {
        test_torznab(host, key)?;
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    let endpoint = format!("{}/settings", base_url.trim_end_matches('/'));
    let mut response = agent
        .post(&endpoint)
        .send_json(serde_json::json!({"action":"get"}))
        .map_err(|error| format!("Не удалось прочитать настройки TorrServer: {error}"))?;
    let mut sets: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректные настройки TorrServer: {error}"))?;
    sets["EnableTorznabSearch"] = serde_json::json!(!host.trim().is_empty());
    sets["TorznabUrls"] = if host.trim().is_empty() {
        serde_json::json!([])
    } else {
        serde_json::json!([{
            "Host": host.trim(),
            "Key": key.trim(),
            "Name": "Prowlarr / Jackett",
            "CatType": "all"
        }])
    };
    agent
        .post(endpoint)
        .send_json(serde_json::json!({"action":"set", "sets":sets}))
        .map_err(|error| format!("Не удалось настроить Jackett/Torznab в TorrServer: {error}"))?;
    Ok(())
}

pub fn test_torznab(host: &str, key: &str) -> Result<(), String> {
    crate::settings::validate_torznab(host, key)?;
    if host.trim().is_empty() {
        return Ok(());
    }
    let host = host.trim();
    let endpoint = if host.to_ascii_lowercase().contains("/api/v2.0/indexers/") {
        format!("{}/api", host.trim_end_matches('/'))
    } else {
        host.to_owned()
    };
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    let encoded_key: String = key
        .trim()
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(15)))
        .build()
        .into();
    let body = agent
        .get(format!("{endpoint}{separator}apikey={encoded_key}&t=caps"))
        .call()
        .map_err(|error| format!("Jackett/Torznab не отвечает: {error}"))?
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Не удалось прочитать ответ Jackett/Torznab: {error}"))?;
    if !body.contains("<caps") {
        return Err("Jackett/Torznab вернул ответ без описания возможностей".into());
    }
    Ok(())
}

pub fn search_all_sources(
    base_url: &str,
    torznab_url: &str,
    query: &str,
) -> Result<Vec<SearchResult>, String> {
    let discovered = torznab_url
        .trim()
        .is_empty()
        .then(|| read_torznab_config(base_url).ok().flatten())
        .flatten();
    let torznab_url = discovered
        .as_ref()
        .map(|(host, _)| host.as_str())
        .unwrap_or(torznab_url);
    let rutor = search_torrserver(base_url, query);
    let torznab =
        (!torznab_url.trim().is_empty()).then(|| search_torznab(base_url, torznab_url, query));
    if let Err(rutor_error) = &rutor {
        if torznab.as_ref().is_none_or(Result::is_err) {
            return Err(torznab
                .and_then(Result::err)
                .map(|error| format!("RuTor: {rutor_error}. Jackett/Torznab: {error}"))
                .unwrap_or_else(|| rutor_error.clone()));
        }
    }
    let mut results = rutor.unwrap_or_default();
    if let Some(Ok(extra)) = torznab {
        let mut seen: std::collections::HashSet<String> = results
            .iter()
            .map(|item| {
                if item.hash.is_empty() {
                    item.magnet.clone()
                } else {
                    item.hash.clone()
                }
            })
            .collect();
        results.extend(extra.into_iter().filter(|item| {
            seen.insert(if item.hash.is_empty() {
                item.magnet.clone()
            } else {
                item.hash.clone()
            })
        }));
    }
    Ok(results)
}

fn search_torrserver_path(
    base_url: &str,
    query: &str,
    path: &str,
    source: &str,
    torrent_origin: Option<&str>,
) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Err("Введите минимум 2 символа".into());
    }
    let encoded: String = query
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let response: serde_json::Value = agent
        .get(format!(
            "{}{}?query={encoded}",
            base_url.trim_end_matches('/'),
            path
        ))
        .call()
        .map_err(|error| format!("Поиск TorrServer не ответил: {error}"))?
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный ответ поиска: {error}"))?;
    let items = response
        .as_array()
        .ok_or("TorrServer вернул некорректный список результатов")?;
    let mut seen = std::collections::HashSet::new();
    Ok(items
        .iter()
        .take(40)
        .filter_map(|item| {
            let raw_magnet = ["Magnet", "magnet", "Link", "link"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                .unwrap_or("")
                .trim()
                .replace("&amp;", "&");
            let raw_magnet = raw_magnet.trim_matches(['\'', '"']);
            let is_magnet = raw_magnet.to_ascii_lowercase().starts_with("magnet:?");
            let magnet_hash = is_magnet.then(|| magnet_info_hash(raw_magnet)).flatten();
            let raw_hash = ["Hash", "hash", "info_hash"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                .and_then(normalize_info_hash);
            let valid_torrent_url = !is_magnet
                && torrent_origin.is_some_and(|origin| valid_torrent_download(raw_magnet, origin));
            let hash = magnet_hash.or(raw_hash).unwrap_or_default();
            if hash.is_empty() && !valid_torrent_url {
                return None;
            }
            let identity = if hash.is_empty() { raw_magnet } else { &hash };
            if !seen.insert(identity.to_owned()) {
                return None;
            }
            let magnet = if is_magnet && !hash.is_empty() || valid_torrent_url {
                raw_magnet.to_owned()
            } else {
                format!("magnet:?xt=urn:btih:{hash}")
            };
            let title = ["Title", "title", "Name", "name"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Без названия")
                .to_owned();
            let size = ["Size", "size"]
                .iter()
                .find_map(|key| item.get(*key))
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string())
                })
                .unwrap_or_else(|| "—".to_owned());
            let seeders = ["Seed", "seed", "seeders"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_i64()))
                .unwrap_or(0)
                .max(0);
            let tracker = ["Tracker", "tracker", "Indexer", "indexer"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                .unwrap_or_default()
                .to_owned();
            Some(SearchResult {
                title,
                magnet,
                hash,
                size,
                seeders,
                source: source.to_owned(),
                tracker,
            })
        })
        .collect())
}

fn valid_torrent_download(link: &str, configured_url: &str) -> bool {
    let Ok(link_uri) = ureq::http::Uri::try_from(link) else {
        return false;
    };
    let Ok(origin_uri) = ureq::http::Uri::try_from(configured_url) else {
        return false;
    };
    if link_uri.scheme_str() != origin_uri.scheme_str()
        || link_uri.authority() != origin_uri.authority()
    {
        return false;
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    let Ok(mut response) = agent.get(link).call() else {
        return false;
    };
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !content_type.contains("bittorrent") && !content_type.contains("octet-stream") {
        return false;
    }
    use std::io::Read;
    let mut first = [0_u8; 1];
    response
        .body_mut()
        .as_reader()
        .read_exact(&mut first)
        .is_ok()
        && first[0] == b'd'
}

pub fn add_magnet(base_url: &str, magnet: &str, title: &str) -> Result<AddResult, String> {
    let link = magnet
        .trim()
        .trim_matches(['\'', '"'])
        .replace("&amp;", "&");
    let hash = if link.to_ascii_lowercase().starts_with("magnet:?") {
        magnet_info_hash(&link).ok_or("Magnet-ссылка не содержит корректный BTIH")?
    } else {
        let uri = ureq::http::Uri::try_from(&link)
            .map_err(|_| "Нужна magnet-ссылка или HTTP(S)-ссылка на torrent-файл")?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
            return Err("Нужна magnet-ссылка или HTTP(S)-ссылка на torrent-файл".into());
        }
        String::new()
    };
    let (_, existing) = read_torrserver(base_url)?;
    if !hash.is_empty() && existing.iter().any(|torrent| torrent.hash == hash) {
        return Ok(AddResult {
            hash,
            already_exists: true,
        });
    }
    let title = if title.trim().is_empty() {
        "Раздача"
    } else {
        title.trim()
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let mut response = agent
        .post(format!("{}/torrents", base_url.trim_end_matches('/')))
        .send_json(serde_json::json!({
            "action": "add", "link": link, "title": title, "save_to_db": true
        }))
        .map_err(|error| format!("Не удалось добавить раздачу: {error}"))?;
    let response_hash = response
        .body_mut()
        .read_json::<serde_json::Value>()
        .ok()
        .and_then(|value| {
            ["hash", "Hash", "info_hash", "torrent_hash"]
                .iter()
                .find_map(|key| value.get(*key).and_then(|value| value.as_str()))
                .and_then(normalize_info_hash)
        })
        .unwrap_or(hash);
    Ok(AddResult {
        hash: response_hash,
        already_exists: false,
    })
}

pub fn remove_torrent(base_url: &str, hash: &str) -> Result<(), String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректный хеш раздачи")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    agent
        .post(format!("{}/torrents", base_url.trim_end_matches('/')))
        .send_json(serde_json::json!({"action": "rem", "hash": hash}))
        .map_err(|error| format!("Не удалось удалить раздачу из TorrServer: {error}"))?;
    Ok(())
}

pub fn read_torrserver(base_url: &str) -> Result<(String, Vec<Torrent>), String> {
    let base_url = base_url.trim().trim_end_matches('/');
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err("Адрес TorrServer должен начинаться с http:// или https://".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(6)))
        .build()
        .into();
    let version = agent
        .get(format!("{base_url}/echo"))
        .call()
        .map_err(|error| format!("TorrServer не отвечает: {error}"))?
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Не удалось прочитать ответ TorrServer: {error}"))?;
    let response: serde_json::Value = agent
        .post(format!("{base_url}/torrents"))
        .send_json(serde_json::json!({"action": "list"}))
        .map_err(|error| format!("Не удалось получить медиатеку: {error}"))?
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный ответ TorrServer: {error}"))?;
    let items = response
        .as_array()
        .or_else(|| response.get("torrents").and_then(|value| value.as_array()))
        .or_else(|| response.get("items").and_then(|value| value.as_array()))
        .ok_or("TorrServer вернул некорректный список раздач")?;
    let torrents = items
        .iter()
        .filter_map(|item| {
            let hash = ["hash", "Hash", "info_hash"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))?;
            let hash = normalize_info_hash(hash)?;
            let title = ["title", "Title", "name", "Name"]
                .iter()
                .find_map(|key| item.get(*key).and_then(|value| value.as_str()))
                .unwrap_or("Без названия")
                .to_owned();
            Some(Torrent { title, hash })
        })
        .collect();
    Ok((version.trim_matches('"').trim().to_owned(), torrents))
}

pub fn normalize_info_hash(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Some(value.to_ascii_lowercase());
    }
    if value.len() != 32 {
        return None;
    }

    let mut bytes = [0_u8; 20];
    let (mut buffer, mut bits, mut index) = (0_u32, 0_u8, 0_usize);
    for byte in value.bytes() {
        let digit = match byte.to_ascii_uppercase() {
            b'A'..=b'Z' => byte.to_ascii_uppercase() - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | u32::from(digit);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes[index] = (buffer >> bits) as u8;
            index += 1;
            buffer &= (1 << bits) - 1;
        }
    }
    if index != bytes.len() {
        return None;
    }
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn magnet_info_hash(input: &str) -> Option<String> {
    let trimmed = input
        .trim()
        .trim_matches(|character| character == '\'' || character == '"');
    let cleaned = trimmed.replace("&amp;", "&");
    let offset = cleaned.to_ascii_lowercase().find("magnet:?")?;
    let query = &cleaned[offset + "magnet:?".len()..];
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if !key.eq_ignore_ascii_case("xt") {
            return None;
        }
        let decoded = percent_decode(value)?;
        let hash = decoded.get("urn:btih:".len()..)?;
        decoded
            .get(.."urn:btih:".len())
            .filter(|prefix| prefix.eq_ignore_ascii_case("urn:btih:"))
            .and_then(|_| normalize_info_hash(hash))
    })
}

pub fn magnet_title(input: &str) -> String {
    let cleaned = input.trim().trim_matches(['\'', '"']).replace("&amp;", "&");
    cleaned
        .split_once('?')
        .map(|(_, query)| query)
        .into_iter()
        .flat_map(|query| query.split('&'))
        .find_map(|part| {
            let (key, value) = part.split_once('=')?;
            key.eq_ignore_ascii_case("dn")
                .then(|| percent_decode(&value.replace('+', " ")))
                .flatten()
        })
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Magnet".into())
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let pair = bytes.get(index + 1..index + 3)?;
            let hex = std::str::from_utf8(pair).ok()?;
            output.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn accepts_hex_and_base32_btih() {
        assert_eq!(normalize_info_hash(&"A".repeat(40)), Some("a".repeat(40)));
        assert_eq!(normalize_info_hash(&"A".repeat(32)), Some("0".repeat(40)));
        assert_eq!(normalize_info_hash("not-a-hash"), None);
    }

    #[test]
    fn reads_encoded_magnet_xt_without_using_display_name() {
        let link = format!("'magnet:?dn=Movie&amp;xt=urn%3Abtih%3A{}'", "B".repeat(40));
        assert_eq!(magnet_info_hash(&link), Some("b".repeat(40)));
        assert_eq!(magnet_info_hash("magnet:?dn=No%20hash"), None);
    }

    #[test]
    fn reads_magnet_display_name_for_os_handler() {
        assert_eq!(
            magnet_title(
                "magnet:?xt=urn:btih:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&dn=Movie%202026"
            ),
            "Movie 2026"
        );
    }

    #[test]
    fn compares_release_versions_without_lexical_errors() {
        assert!(newer_version("0.2.0", "0.1.11"));
        assert!(!newer_version("0.1.9", "0.1.11"));
        assert!(!newer_version("0.1.11", "0.1.11"));
    }

    #[test]
    fn reads_local_torrserver_list() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (path, body) in [
                ("/echo", "MatriX.135".to_owned()),
                (
                    "/torrents",
                    format!(r#"[{{"hash":"{}","title":"Фильм"}}]"#, "a".repeat(40)),
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 2048];
                let mut size = stream.read(&mut request).unwrap();
                if path == "/torrents" {
                    let header_end = request[..size]
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .unwrap()
                        + 4;
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    while size < header_end + length {
                        size += stream.read(&mut request[size..]).unwrap();
                    }
                }
                assert!(String::from_utf8_lossy(&request[..size]).contains(path));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        let (version, torrents) = read_torrserver(&format!("http://{address}")).unwrap();
        server.join().unwrap();
        assert_eq!(version, "MatriX.135");
        assert_eq!(torrents[0].title, "Фильм");
        assert_eq!(torrents[0].hash, "a".repeat(40));
    }

    #[test]
    fn searches_only_valid_magnets_and_encodes_cyrillic_query() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let size = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..size]).contains("/search?query=%D1%84"));
            let body = format!(
                r#"[{{"Title":"Фильм","Hash":"{}","Seed":5}},{{"Title":"Дубликат","Hash":"{}"}},{{"Title":"Bad","Magnet":"magnet:?dn=NoHash"}}]"#,
                "b".repeat(40),
                "b".repeat(40)
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let results = search_torrserver(&format!("http://{address}"), "фильм").unwrap();
        server.join().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Фильм");
        assert_eq!(results[0].seeders, 5);
        assert_eq!(
            results[0].magnet,
            format!("magnet:?xt=urn:btih:{}", "b".repeat(40))
        );
    }

    #[test]
    fn labels_jackett_torznab_results() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let size = stream.read(&mut request).unwrap();
            assert!(
                String::from_utf8_lossy(&request[..size]).contains("/torznab/search/?query=movie")
            );
            let body = format!(
                r#"[{{"Title":"Movie","Hash":"{}","Tracker":"Example"}}]"#,
                "c".repeat(40)
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let base = format!("http://{address}");
        let results = search_torznab(
            &base,
            &format!("{base}/api/v2.0/indexers/all/results/torznab"),
            "movie",
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "Jackett");
        assert_eq!(results[0].tracker, "Example");
    }

    #[test]
    fn discovers_existing_torznab_configuration_from_torrserver() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let size = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..size]);
                let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..headers_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                if request.len() >= headers_end + 4 + content_length {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);
            assert!(request.starts_with("POST /settings "));
            let body = r#"{"EnableTorznabSearch":true,"TorznabUrls":[{"Host":"http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/","Key":"secret"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let config = read_torznab_config(&format!("http://{address}"))
            .unwrap()
            .unwrap();
        server.join().unwrap();
        assert_eq!(
            config.0,
            "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/"
        );
        assert_eq!(config.1, "secret");
    }

    #[test]
    fn validates_jackett_caps_before_saving_configuration() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let size = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.contains("/api/v2.0/indexers/all/results/torznab/api?"));
            assert!(request.contains("apikey=secret&t=caps"));
            let body = "<?xml version=\"1.0\"?><caps><searching/></caps>";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        test_torznab(
            &format!("http://{address}/api/v2.0/indexers/all/results/torznab/"),
            "secret",
        )
        .unwrap();
        server.join().unwrap();
    }

    #[test]
    fn accepts_only_same_origin_bencoded_torrent_downloads() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-bittorrent\r\nContent-Length: 1\r\nConnection: close\r\n\r\nd")
                .unwrap();
        });
        let origin = format!("http://{address}/api/torznab");
        assert!(valid_torrent_download(
            &format!("http://{address}/download?id=1"),
            &origin
        ));
        server.join().unwrap();
        assert!(!valid_torrent_download(
            "https://example.com/file.torrent",
            &origin
        ));
    }

    #[test]
    fn adds_magnet_to_local_torrserver_once() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (path, body) in [
                ("/echo", "MatriX.144.3"),
                ("/torrents", "[]"),
                ("/torrents", "{}"),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 2048];
                let mut size = stream.read(&mut request).unwrap();
                if path == "/torrents" {
                    let header_end = request[..size]
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .unwrap()
                        + 4;
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    while size < header_end + length {
                        size += stream.read(&mut request[size..]).unwrap();
                    }
                    let payload: serde_json::Value =
                        serde_json::from_slice(&request[header_end..header_end + length]).unwrap();
                    if body == "{}" {
                        assert_eq!(payload["action"], "add");
                        assert_eq!(payload["save_to_db"], true);
                        assert_eq!(payload["title"], "Фильм");
                        assert!(!payload["link"].as_str().unwrap().contains("&amp;"));
                    } else {
                        assert_eq!(payload["action"], "list");
                    }
                }
                assert!(String::from_utf8_lossy(&request[..size]).contains(path));
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        let result = add_magnet(
            &format!("http://{address}"),
            &format!("magnet:?xt=urn:btih:{}&amp;dn=Movie", "c".repeat(40)),
            "Фильм",
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(result.hash, "c".repeat(40));
        assert!(!result.already_exists);
    }

    #[test]
    fn keeps_original_video_file_ids_and_stream_names() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let hash = "d".repeat(40);
        let expected_hash = hash.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let mut size = stream.read(&mut request).unwrap();
            let header_end = request[..size]
                .windows(4)
                .position(|part| part == b"\r\n\r\n")
                .unwrap()
                + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length: usize = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            while size < header_end + length {
                size += stream.read(&mut request[size..]).unwrap();
            }
            assert!(String::from_utf8_lossy(&request[..size]).contains("POST /torrents"));
            let body = format!(
                r#"[{{"hash":"{expected_hash}","file_stats":[{{"id":7,"path":"Шоу/Сезон 2/Серия 3.mkv","length":123456789}},{{"id":8,"path":"readme.txt","length":12}}]}}]"#
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let files = torrent_video_files(&format!("http://{address}"), &hash).unwrap();
        server.join().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, 7);
        assert_eq!(files[0].name, "Серия 3.mkv");
        let url = stream_url("http://127.0.0.1:8090", &hash, &files[0]).unwrap();
        assert!(url.contains("index=7&play"));
        assert!(url.contains("%D0%A1%D0%B5%D1%80%D0%B8%D1%8F"));
    }

    #[test]
    fn orders_episodes_naturally_without_changing_ids() {
        assert!(natural_path_cmp("Сезон 2/Серия 2.mkv", "Сезон 2/Серия 10.mkv").is_lt());
        assert!(natural_path_cmp("Season 9/E1.mkv", "Season 10/E1.mkv").is_lt());
    }

    #[test]
    fn reads_matrix_saved_files_without_waiting_for_stream_details() {
        let saved = serde_json::json!({
            "hash": "ae30a8d74972b410afaee63081cb9bd902eeff31",
            "data": r#"{"TorrServer":{"Files":[{"id":1,"path":"TMNT (2007)-60fps.mkv","length":4152182854}]}}"#
        });
        let files = parse_video_files(&saved);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, 1);
        assert_eq!(files[0].length, 4_152_182_854);
    }

    #[test]
    fn stream_probe_reports_http_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let size = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.contains("Range: bytes=0-0") || request.contains("range: bytes=0-0"));
            stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let file = VideoFile {
            id: 1,
            name: "movie.mkv".into(),
            path: "movie.mkv".into(),
            length: 100,
        };
        let result = probe_stream(&format!("http://{address}"), &"a".repeat(40), &file);
        server.join().unwrap();
        assert!(result.unwrap_err().contains("HTTP 500"));
    }

    #[test]
    fn retries_transient_details_error_before_opening_card() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for (status, body) in [
                ("200 OK", "[]"),
                ("500 Internal Server Error", "{}"),
                (
                    "200 OK",
                    r#"{"file_stats":[{"id":9,"path":"S1E2.mkv","length":42}]}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 2048];
                let size = stream.read(&mut request).unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..size]).contains(if body == "[]" {
                        "POST /torrents"
                    } else {
                        "/stream/details?link="
                    })
                );
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        let files = torrent_video_files(&format!("http://{address}"), &"a".repeat(40)).unwrap();
        server.join().unwrap();
        assert_eq!(files[0].id, 9);
    }

    #[test]
    fn removes_only_the_selected_hash() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let hash = "a".repeat(40);
        let expected = hash.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0_u8; 4096];
            let mut size = stream.read(&mut request).unwrap();
            let header_end = request[..size]
                .windows(4)
                .position(|part| part == b"\r\n\r\n")
                .unwrap()
                + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length: usize = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .unwrap()
                .parse()
                .unwrap();
            while size < header_end + length {
                size += stream.read(&mut request[size..]).unwrap();
            }
            assert!(String::from_utf8_lossy(&request[..header_end]).contains("POST /torrents"));
            let payload: serde_json::Value =
                serde_json::from_slice(&request[header_end..header_end + length]).unwrap();
            assert_eq!(payload["hash"], expected);
            assert_eq!(payload["action"], "rem");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .unwrap();
        });
        remove_torrent(&format!("http://{address}"), &hash).unwrap();
        server.join().unwrap();
        assert!(remove_torrent("http://127.0.0.1:8090", "bad").is_err());
    }
}
