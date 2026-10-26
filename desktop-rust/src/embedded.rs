//! Shared in-process torrent engine.
use crate::{magnet_info_hash, normalize_info_hash, SearchResult, Torrent, VideoFile};
use librqbit::{
    AddTorrent, AddTorrentOptions, Api, Session, SessionOptions, SessionPersistenceConfig,
};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: both buffers are NUL-terminated and live for the duration of the call.
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

fn reset_payload_cache(root: &Path) -> Result<std::path::PathBuf, String> {
    let downloads = root.join("downloads");
    if let Ok(metadata) = std::fs::symlink_metadata(&downloads) {
        if metadata.file_type().is_symlink() {
            return Err("Папка буфера librqbit не должна быть ссылкой".into());
        }
        std::fs::remove_dir_all(&downloads).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    Ok(downloads)
}

fn pause_saved_session(path: &Path) -> Result<(), String> {
    let input = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    if input.metadata().map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
        return Err("Слишком большая сохранённая сессия librqbit".into());
    }
    let mut value: serde_json::Value = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let torrents = value
        .get_mut("torrents")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("Некорректная сессия librqbit")?;
    for item in torrents.values_mut() {
        let paused = item
            .get_mut("is_paused")
            .filter(|v| v.is_boolean())
            .ok_or("Некорректное состояние раздачи librqbit")?;
        *paused = serde_json::json!(true);
    }
    let temporary = path.with_extension("json.part");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace_file(&temporary, path).map_err(|e| e.to_string())
}

pub struct Engine {
    api: Api,
    endpoint: String,
    payload_root: PathBuf,
    http: tokio::task::JoinHandle<Result<(), String>>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.http.abort();
        self.api.session().cancellation_token().cancel();
    }
}

impl Engine {
    pub async fn start(profile: &Path) -> Result<Arc<Self>, String> {
        let root = profile.join("rqbit");
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        pause_saved_session(&root.join("session/session.json"))?;
        let payload_root = reset_payload_cache(&root)?;
        let session = Session::new_with_opts(
            payload_root.clone(),
            SessionOptions {
                dht: Some(librqbit::DhtSessionConfig {
                    persistence: None,
                    ..Default::default()
                }),
                persistence: Some(SessionPersistenceConfig::Json {
                    folder: Some(root.join("session")),
                }),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("librqbit: {e:#}"))?;
        Self::serve(session, payload_root).await
    }

    async fn serve(session: Arc<Session>, payload_root: PathBuf) -> Result<Arc<Self>, String> {
        let api = Api::new(session, None, None);
        let listener = librqbit_dualstack_sockets::TcpListener::bind_tcp(
            std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            librqbit_dualstack_sockets::BindOpts {
                request_dualstack: false,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let address = listener.bind_addr();
        let endpoint = format!("http://127.0.0.1:{}", address.port());
        let server = librqbit::http_api::HttpApi::new(
            api.clone(),
            Some(librqbit::http_api::HttpApiOptions {
                read_only: true,
                ..Default::default()
            }),
        );
        let http = tokio::spawn(async move {
            server
                .make_http_api_and_run(listener, None)
                .await
                .map_err(|e| format!("{e:#}"))
        });
        if tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if tokio::net::TcpStream::connect(address).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_err()
        {
            if http.is_finished() {
                return Err(match http.await {
                    Ok(Err(error)) => format!("Локальный HTTP-поток librqbit завершился: {error}"),
                    Ok(Ok(())) => "Локальный HTTP-поток librqbit неожиданно завершился".into(),
                    Err(error) => format!("Локальный HTTP-поток librqbit: {error}"),
                });
            }
            http.abort();
            return Err("Локальный HTTP-поток librqbit не запустился за 2 секунды".into());
        }
        Ok(Arc::new(Self {
            api,
            endpoint,
            payload_root,
            http,
        }))
    }

    pub async fn stop(&self) {
        self.http.abort();
        self.api.session().stop().await;
    }

    pub fn healthy(&self) -> bool {
        !self.http.is_finished()
    }

    pub async fn remove(&self, hash: &str) -> Result<(), String> {
        self.api
            .session()
            .delete(Self::id(hash)?, true)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn clear_payload(&self, hash: &str) -> Result<(), String> {
        let hash = normalize_info_hash(hash).ok_or("Некорректная раздача")?;
        let session = self.api.session();
        let handle = session.get(Self::id(&hash)?).ok_or("Раздача не найдена")?;
        session.pause(&handle).await.map_err(|e| e.to_string())?;
        let path = self.payload_root.join(&hash);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Err("Папка буфера раздачи не должна быть ссылкой".into())
            }
            Ok(_) => std::fs::remove_dir_all(path).map_err(|e| e.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn list(&self) -> Vec<Torrent> {
        self.api
            .api_torrent_list()
            .torrents
            .into_iter()
            .map(|item| Torrent {
                title: item.name.unwrap_or_else(|| item.info_hash.clone()),
                hash: item.info_hash,
            })
            .collect()
    }

    pub async fn add(&self, magnet: &str) -> Result<(), String> {
        let hash = magnet_info_hash(magnet).ok_or("Нужна корректная magnet-ссылка")?;
        // Metadata only until the user explicitly starts a chosen file.
        let response = tokio::time::timeout(
            Duration::from_secs(60),
            self.api.session().add_torrent(
                AddTorrent::from_url(magnet),
                Some(AddTorrentOptions {
                    paused: true,
                    overwrite: true,
                    sub_folder: Some(hash),
                    only_files: Some(vec![]),
                    ..Default::default()
                }),
            ),
        )
        .await
        .map_err(|_| "librqbit: не получен список файлов за 60 секунд".to_owned())?
        .map_err(|e| format!("librqbit: {e:#}"))?;
        if let Some(handle) = response.into_handle() {
            tokio::time::timeout(Duration::from_secs(60), handle.wait_until_initialized())
                .await
                .map_err(|_| "librqbit: подготовка файлов превысила 60 секунд")?
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Import bounded .torrent metadata, still paused with no selected payload.
    pub async fn add_torrent_file(&self, bytes: Vec<u8>) -> Result<String, String> {
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Слишком большой torrent-файл".into());
        }
        let hash = librqbit::torrent_from_bytes(&bytes)
            .map_err(|e| e.to_string())?
            .info_hash
            .as_string();
        let response = tokio::time::timeout(
            Duration::from_secs(60),
            self.api.session().add_torrent(
                AddTorrent::TorrentFileBytes(bytes.into()),
                Some(AddTorrentOptions {
                    paused: true,
                    overwrite: true,
                    sub_folder: Some(hash),
                    only_files: Some(vec![]),
                    ..Default::default()
                }),
            ),
        )
        .await
        .map_err(|_| "Не получены метаданные за 60 секунд")?
        .map_err(|e| e.to_string())?;
        let handle = response.into_handle().ok_or("Раздача не сохранена")?;
        tokio::time::timeout(Duration::from_secs(60), handle.wait_until_initialized())
            .await
            .map_err(|_| "Не завершена подготовка раздачи за 60 секунд")?
            .map_err(|e| e.to_string())?;
        Ok(handle.info_hash().as_string())
    }

    fn id(hash: &str) -> Result<librqbit::api::TorrentIdOrHash, String> {
        let hash = normalize_info_hash(hash).ok_or("Некорректная раздача")?;
        librqbit::api::TorrentIdOrHash::parse(&hash).map_err(|e| e.to_string())
    }

    pub fn files(&self, hash: &str) -> Result<Vec<VideoFile>, String> {
        let details = self
            .api
            .api_torrent_details(Self::id(hash)?)
            .map_err(|e| e.to_string())?;
        let mut files = details
            .files
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .filter_map(|(index, file)| {
                let name = file.components.last()?.clone();
                let extension = name.rsplit('.').next()?.to_ascii_lowercase();
                if !matches!(
                    extension.as_str(),
                    "mkv" | "mp4" | "avi" | "mov" | "m4v" | "webm" | "ts" | "m2ts"
                ) {
                    return None;
                }
                Some(VideoFile {
                    id: index as i64 + 1,
                    name,
                    path: file.name,
                    length: file.length,
                })
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| crate::natural_path_cmp(&a.path, &b.path));
        Ok(files)
    }

    pub fn is_active(&self, hash: &str) -> Result<bool, String> {
        Ok(self
            .api
            .mgr_handle(Self::id(hash)?)
            .map_err(|e| e.to_string())?
            .live()
            .is_some())
    }

    pub async fn pause(&self, hash: &str) -> Result<(), String> {
        self.api
            .api_torrent_action_pause(Self::id(hash)?)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn play_url(&self, hash: &str, file: &VideoFile) -> Result<String, String> {
        if self.http.is_finished() {
            return Err(
                "Локальный HTTP-поток librqbit остановлен. Перезапустите приложение.".into(),
            );
        }
        if !self.files(hash)?.iter().any(|candidate| candidate == file) {
            return Err("Файл не найден в раздаче".into());
        }
        let index = usize::try_from(file.id - 1).map_err(|e| e.to_string())?;
        let id = Self::id(hash)?;
        let handle = self.api.mgr_handle(id).map_err(|e| e.to_string())?;
        tokio::time::timeout(Duration::from_secs(60), handle.wait_until_initialized())
            .await
            .map_err(|_| "librqbit: подготовка раздачи к запуску превысила 60 секунд".to_owned())?
            .map_err(|e| format!("librqbit: раздача не готова к запуску: {e:#}"))?;
        self.api
            .api_torrent_action_update_only_files(id, &[index].into())
            .await
            .map_err(|e| e.to_string())?;
        if handle.live().is_none() {
            self.api
                .api_torrent_action_start(id)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(format!("{}/torrents/{hash}/stream/{index}", self.endpoint))
    }
}

const INDEX_URL: &str = "https://releases.yourok.ru/torr/rutor.ls";
const MAX_INDEX: u64 = 80 * 1024 * 1024;

/// Fetches the public index, never uploads the query, library, or history.
pub fn search(profile: &Path, query: &str) -> Result<Vec<SearchResult>, String> {
    let words = tokens(query);
    if query.trim().chars().count() < 2
        || query.chars().count() > 200
        || words.is_empty()
        || words.iter().map(String::len).sum::<usize>() < 2
    {
        return Err("Введите название (2–200 символов)".into());
    }
    let root = profile.join("rqbit");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let path = root.join("rutor.ls");
    if !path.exists() {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(180)))
            .build()
            .into();
        let mut response = agent
            .get(INDEX_URL)
            .call()
            .map_err(|e| format!("Индекс поиска: {e}"))?;
        let temporary = root.join("rutor.ls.part");
        let mut output = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        let length = std::io::copy(
            &mut response.body_mut().as_reader().take(MAX_INDEX + 1),
            &mut output,
        )
        .map_err(|e| e.to_string())?;
        drop(output);
        if length > MAX_INDEX {
            return Err("Индекс превышает допустимый размер".into());
        }
        let result = search_index(
            std::fs::File::open(&temporary).map_err(|e| e.to_string())?,
            &words,
        )?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
        return Ok(result);
    }
    search_index(
        std::fs::File::open(path).map_err(|e| e.to_string())?,
        &words,
    )
}

fn tokens(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct IndexEntry {
    title: String,
    name: String,
    names: Option<Vec<String>>,
    size: String,
    hash: String,
    magnet: String,
    seed: i64,
    tracker: String,
}

fn search_index(reader: impl Read, words: &[String]) -> Result<Vec<SearchResult>, String> {
    struct Matches<'a>(&'a [String]);
    impl<'de> Visitor<'de> for Matches<'_> {
        type Value = Vec<SearchResult>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("массив поискового индекса")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut found = Vec::new();
            while let Some(item) = seq.next_element::<IndexEntry>()? {
                let title = if item.title.is_empty() {
                    item.name.clone()
                } else {
                    item.title
                };
                let searchable = tokens(&format!(
                    "{title} {} {}",
                    item.name,
                    item.names.unwrap_or_default().join(" ")
                ));
                if !self.0.iter().all(|word| searchable.contains(word)) {
                    continue;
                }
                let Some(hash) =
                    magnet_info_hash(&item.magnet).or_else(|| normalize_info_hash(&item.hash))
                else {
                    continue;
                };
                if found.iter().any(|item: &SearchResult| item.hash == hash) {
                    continue;
                }
                found.push(SearchResult {
                    title,
                    magnet: format!("magnet:?xt=urn:btih:{hash}"),
                    hash,
                    size: item.size,
                    seeders: item.seed.max(0),
                    source: "Локальный индекс".into(),
                    tracker: item.tracker,
                });
                if found.len() > 120 {
                    found.sort_by(|a, b| {
                        b.seeders
                            .cmp(&a.seeders)
                            .then_with(|| a.title.cmp(&b.title))
                    });
                    found.truncate(60);
                }
            }
            found.sort_by(|a, b| {
                b.seeders
                    .cmp(&a.seeders)
                    .then_with(|| a.title.cmp(&b.title))
            });
            found.truncate(60);
            Ok(found)
        }
    }
    // Stream decompression/deserialization: never load the entire catalogue into RAM.
    let decoder = flate2::read::DeflateDecoder::new(reader).take(512 * 1024 * 1024);
    let mut json = serde_json::Deserializer::from_reader(std::io::BufReader::new(decoder));
    let result = (&mut json)
        .deserialize_seq(Matches(words))
        .map_err(|e| format!("Некорректный индекс: {e}"))?;
    json.end()
        .map_err(|e| format!("Некорректный конец индекса: {e}"))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_removes_stale_payload_cache() {
        let root = std::env::temp_dir().join(format!(
            "pc-rqbit-cache-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let stale = root.join("downloads/hash/video.mkv");
        std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
        std::fs::write(&stale, b"watched content").unwrap();
        let downloads = reset_payload_cache(&root).unwrap();
        assert!(downloads.is_dir());
        assert!(std::fs::read_dir(&downloads).unwrap().next().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn natural_episode_order_keeps_original_file_ids() {
        let mut files = [
            "Show/S1E10.mkv",
            "Show/S2E1.mkv",
            "Show/S1E2.mkv",
            "Show/S1E1.mkv",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, path)| VideoFile {
            id: i as i64 + 1,
            name: path.into(),
            path: path.into(),
            length: 1,
        })
        .collect::<Vec<_>>();
        files.sort_by(|a, b| crate::natural_path_cmp(&a.path, &b.path));
        assert_eq!(
            files.iter().map(|file| file.id).collect::<Vec<_>>(),
            [4, 3, 1, 2]
        );
    }
    #[test]
    fn restoration_is_paused_without_erasing_saved_files() {
        let root = std::env::temp_dir().join(format!(
            "pc-rqbit-restore-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("session.json");
        let original = serde_json::json!({"torrents":{"7":{"is_paused":false,"only_files":[2],"info_hash":"test","output_folder":"private"}}});
        std::fs::write(&path, original.to_string()).unwrap();
        pause_saved_session(&path).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["torrents"]["7"]["is_paused"], true);
        assert_eq!(
            saved["torrents"]["7"]["only_files"],
            original["torrents"]["7"]["only_files"]
        );
        assert_eq!(saved["torrents"]["7"]["output_folder"], "private");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn repeat_launch_preserves_real_file_id_and_serves_open_http_range() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let root = std::env::temp_dir().join(format!(
                "pc-rqbit-stream-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            let session = Session::new_with_opts(
                root.clone(),
                SessionOptions {
                    dht: None,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let mut torrent =
                b"d4:infod6:lengthi4e4:name8:test.mp412:piece lengthi16384e6:pieces20:".to_vec();
            torrent.extend([
                0xa9, 0x4a, 0x8f, 0xe5, 0xcc, 0xb1, 0x9b, 0xa6, 0x1c, 0x4c, 0x08, 0x73, 0xd3, 0x91,
                0xe9, 0x87, 0x98, 0x2f, 0xbb, 0xd3,
            ]);
            torrent.extend(b"ee");
            let hash = librqbit::torrent_from_bytes(&torrent)
                .unwrap()
                .info_hash
                .as_string();
            let payload = root.join(&hash);
            std::fs::create_dir(&payload).unwrap();
            std::fs::write(payload.join("test.mp4"), b"test").unwrap();
            let handle = session
                .add_torrent(
                    AddTorrent::TorrentFileBytes(torrent.into()),
                    Some(AddTorrentOptions {
                        paused: true,
                        overwrite: true,
                        sub_folder: Some(hash.clone()),
                        ..Default::default()
                    }),
                )
                .await
                .unwrap()
                .into_handle()
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), handle.wait_until_initialized())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(handle.info_hash().as_string(), hash);
            let engine = Engine::serve(session.clone(), root.clone()).await.unwrap();
            let files = engine.files(&hash).unwrap();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].id, 1);
            let first = engine.play_url(&hash, &files[0]).await.unwrap();
            tokio::time::timeout(Duration::from_secs(5), handle.wait_until_completed())
                .await
                .unwrap()
                .unwrap();
            let second = engine.play_url(&hash, &files[0]).await.unwrap();
            assert_eq!(first, second);
            tokio::task::spawn_blocking(move || {
                let agent: ureq::Agent = ureq::Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(4)))
                    .build()
                    .into();
                let mut reply = agent
                    .get(&second)
                    .header("Range", "bytes=1-")
                    .call()
                    .unwrap();
                assert_eq!(reply.status().as_u16(), 206);
                assert_eq!(reply.body_mut().read_to_vec().unwrap(), b"est");
            })
            .await
            .unwrap();
            engine.clear_payload(&hash).await.unwrap();
            assert!(!payload.exists());
            assert_eq!(engine.files(&hash).unwrap(), files);
            assert!(engine.play_url(&hash, &files[0]).await.is_ok());
            session.stop().await;
            drop(engine);
            drop(handle);
            drop(session);
            std::fs::remove_dir(root).unwrap();
        });
    }
    #[test]
    fn search_is_local_matches_words_and_rejects_bad_magnets() {
        use std::io::Write;
        let raw = serde_json::json!([
            {"Title":"Тачки / Cars (2006)","Hash":"a".repeat(40),"Seed":12},
            {"Title":"Тачки 2 (2011)","Hash":"b".repeat(40),"Seed":5},
            {"Title":"Натачки","Hash":"c".repeat(40),"Seed":500},
            {"Title":"Тачки","Hash":"invalid"}
        ]);
        let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), Default::default());
        encoder.write_all(raw.to_string().as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let result = search_index(compressed.as_slice(), &tokens("ТАЧКИ")).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].seeders, 12);
        assert!(search_index(b"invalid".as_slice(), &tokens("Тачки")).is_err());
    }
}
