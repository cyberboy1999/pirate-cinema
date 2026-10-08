use crate::normalize_info_hash;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Eq)]
pub struct FileHistory {
    pub file_index: i64,
    pub file_name: String,
    pub file_path: Option<String>,
    pub playback_timecode: Option<i64>,
    pub playback_duration: Option<i64>,
    pub is_watched: bool,
    pub launch_count: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecentPlayback {
    pub torrent_hash: String,
    pub file_index: i64,
    pub file_name: String,
    pub playback_timecode: i64,
    pub playback_duration: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaybackPreferences {
    pub season: Option<u8>,
    pub episode: Option<u16>,
    pub auto_next: bool,
}

pub struct HistoryStore {
    db: Connection,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImportSummary {
    pub files: usize,
    pub media_types: usize,
    pub audio_tracks: usize,
    pub metadata: usize,
    pub posters: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaMetadata {
    pub title: String,
    pub overview: Option<String>,
    pub year: Option<i64>,
    pub rating: Option<f64>,
    pub poster_file: Option<String>,
    pub genres: Vec<String>,
}

fn parse_genres(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw)
        .unwrap_or_default()
        .into_iter()
        .map(|genre| genre.trim().to_owned())
        .filter(|genre| !genre.is_empty() && genre.chars().count() <= 80)
        .take(12)
        .collect()
}

fn valid_poster_name(name: &str) -> bool {
    name.strip_suffix(".jpg").is_some_and(|stem| {
        !stem.is_empty()
            && stem
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    })
}

pub fn create_local_backup(
    history_path: &Path,
    settings_path: Option<&Path>,
    torrserver_dir: Option<&Path>,
    backup_root: &Path,
) -> Result<PathBuf, String> {
    if !history_path.is_file() {
        return Err("База истории Rust недоступна".into());
    }
    std::fs::create_dir_all(backup_root).map_err(|error| error.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let target = backup_root.join(format!("backup-{stamp}"));
    std::fs::create_dir(&target).map_err(|error| error.to_string())?;
    HistoryStore::open(history_path)
        .map_err(|error| error.to_string())?
        .snapshot_to(&target.join("history.sqlite"))?;
    let settings_target = target.join("settings.json");
    match settings_path.filter(|path| path.is_file()) {
        Some(path) => {
            let mut settings: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            if let Some(object) = settings.as_object_mut() {
                object.remove("kinopoisk_api_key");
            }
            std::fs::write(&settings_target, settings.to_string())
                .map_err(|error| error.to_string())?;
        }
        None => {
            std::fs::write(
                &settings_target,
                serde_json::json!({
                    "torrserver_url": crate::DEFAULT_TORRSERVER_URL,
                    "player_type": "mpv",
                    "player_path": ""
                })
                .to_string(),
            )
            .map_err(|error| error.to_string())?;
        }
    }
    if let Some(root) = history_path.parent() {
        let posters = root.join("cache").join("posters");
        if posters.is_dir() {
            for entry in std::fs::read_dir(&posters).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                let name = entry.file_name();
                let Some(name) = name.to_str().filter(|name| valid_poster_name(name)) else {
                    continue;
                };
                if !entry
                    .file_type()
                    .map_err(|error| error.to_string())?
                    .is_file()
                {
                    continue;
                }
                if entry.metadata().map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024 {
                    continue;
                }
                let destination = target.join("cache").join("posters");
                std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
                std::fs::copy(entry.path(), destination.join(name))
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    if let Some(source) = torrserver_dir.filter(|path| path.is_dir()) {
        let destination = target.join("torrserver");
        for name in [
            "config.db",
            "settings.json",
            "viewed.json",
            "trackers.txt",
            "rutor.ls",
        ] {
            let file = source.join(name);
            if !file.is_file() {
                continue;
            }
            std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
            std::fs::copy(&file, destination.join(name)).map_err(|error| error.to_string())?;
        }
    }
    std::fs::write(
        target.join("backup.json"),
        serde_json::json!({
            "application": "Pirate Cinema Rust",
            "format": 1,
        "scope": "history-settings-posters-and-torrserver"
        })
        .to_string(),
    )
    .map_err(|error| error.to_string())?;
    Ok(target)
}

impl HistoryStore {
    pub fn integrity_check(&self) -> Result<(), String> {
        let result: String = self
            .db
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if result == "ok" {
            Ok(())
        } else {
            Err(format!("SQLite сообщает: {result}"))
        }
    }

    pub fn restore_local_backup(
        &mut self,
        source: &Path,
        settings_path: Option<&Path>,
        profile_root: &Path,
        torrserver_dir: Option<&Path>,
    ) -> Result<usize, String> {
        let manifest_path = source.join("backup.json");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path)
                .map_err(|_| "В выбранной папке нет backup.json".to_owned())?,
        )
        .map_err(|_| "Некорректный файл backup.json".to_owned())?;
        if manifest.get("application").and_then(|value| value.as_str())
            != Some("Pirate Cinema Rust")
            || manifest.get("format").and_then(|value| value.as_i64()) != Some(1)
        {
            return Err("Это не резервная копия Pirate Cinema Rust формата 1".into());
        }
        let database = source.join("history.sqlite");
        let source_db = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("Не удалось открыть резервную базу: {error}"))?;
        let integrity: String = source_db
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| format!("Не удалось проверить резервную базу: {error}"))?;
        if integrity != "ok" {
            return Err("Резервная база SQLite повреждена".into());
        }
        for table in [
            "media_file_history",
            "media_types",
            "media_audio_tracks",
            "media_metadata",
        ] {
            let exists: bool = source_db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [table],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if !exists {
                return Err(format!("В резервной базе отсутствует таблица {table}"));
            }
        }
        drop(source_db);
        let settings = std::fs::read_to_string(source.join("settings.json"))
            .map_err(|_| "В резервной копии нет settings.json".to_owned())?;
        serde_json::from_str::<serde_json::Value>(&settings)
            .map_err(|_| "Некорректный settings.json в резервной копии".to_owned())?;

        self.db
            .restore(
                rusqlite::MAIN_DB,
                &database,
                None::<fn(rusqlite::backup::Progress)>,
            )
            .map_err(|error| format!("Не удалось восстановить SQLite: {error}"))?;
        if let Some(path) = settings_path {
            let parent = path.parent().ok_or("Некорректный путь настроек")?;
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            std::fs::write(path, settings).map_err(|error| error.to_string())?;
        }
        let mut posters = 0;
        let source_posters = source.join("cache").join("posters");
        if source_posters.is_dir() {
            let destination = profile_root.join("cache").join("posters");
            for entry in std::fs::read_dir(source_posters).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !valid_poster_name(&name)
                    || !entry
                        .file_type()
                        .map_err(|error| error.to_string())?
                        .is_file()
                    || entry.metadata().map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024
                {
                    continue;
                }
                std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
                std::fs::copy(entry.path(), destination.join(name))
                    .map_err(|error| error.to_string())?;
                posters += 1;
            }
        }
        if let (Some(destination), true) = (torrserver_dir, source.join("torrserver").is_dir()) {
            std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
            for name in [
                "config.db",
                "settings.json",
                "viewed.json",
                "trackers.txt",
                "rutor.ls",
            ] {
                let file = source.join("torrserver").join(name);
                if file.is_file() {
                    std::fs::copy(&file, destination.join(name))
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        Ok(posters)
    }

    /// Import only missing rows. The Electron database is opened read-only and never changed.
    pub fn import_electron(
        &mut self,
        source_path: &Path,
        poster_dir: Option<&Path>,
    ) -> Result<ImportSummary, String> {
        if !source_path.is_file() {
            return Err(format!(
                "База Electron не найдена: {}",
                source_path.display()
            ));
        }
        let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("Не удалось прочитать базу Electron: {error}"))?;
        source
            .busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        let file_rows = {
            let mut query = source
                .prepare(
                    "SELECT torrent_hash,file_index,file_name,file_path,first_played_at,
                playback_timecode,playback_duration,is_watched,last_played_at,launch_count
                FROM media_file_history",
                )
                .map_err(|error| error.to_string())?;
            let rows = query
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                })
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?;
            rows
        };
        let media_rows = {
            let has_genres: bool = source.query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('media_items') WHERE name='genres_json')",
                [], |row| row.get(0),
            ).map_err(|error| error.to_string())?;
            let mut query = source
                .prepare(if has_genres {
                    "SELECT torrent_hash,media_type,audio_track_id,title,overview,year,rating,poster_path,genres_json FROM media_items"
                } else {
                    "SELECT torrent_hash,media_type,audio_track_id,title,overview,year,rating,poster_path,'[]' FROM media_items"
                })
                .map_err(|error| error.to_string())?;
            let rows = query
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<f64>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                })
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?;
            rows
        };
        let tx = self.db.transaction().map_err(|error| error.to_string())?;
        let mut summary = ImportSummary {
            files: 0,
            media_types: 0,
            audio_tracks: 0,
            metadata: 0,
            posters: 0,
        };
        let mut posters_to_copy = Vec::new();
        for (hash, index, name, path, first, time, duration, viewed, last, count) in file_rows {
            let Some(hash) = normalize_info_hash(&hash) else {
                continue;
            };
            if index < 0 || name.trim().is_empty() {
                continue;
            }
            summary.files += tx
                .execute(
                    "INSERT OR IGNORE INTO media_file_history
                 (torrent_hash,file_index,file_name,file_path,first_played_at,playback_timecode,
                  playback_duration,is_watched,last_played_at,launch_count)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                    params![hash, index, name, path, first, time, duration, viewed, last, count],
                )
                .map_err(|error| error.to_string())?;
        }
        for (hash, kind, track, title, overview, year, rating, poster_file, genres_json) in
            media_rows
        {
            let Some(hash) = normalize_info_hash(&hash) else {
                continue;
            };
            if matches!(kind.as_str(), "movie" | "series") {
                summary.media_types += tx
                    .execute(
                        "INSERT OR IGNORE INTO media_types(torrent_hash,media_type) VALUES(?1,?2)",
                        params![hash, kind],
                    )
                    .map_err(|error| error.to_string())?;
            }
            if let Some(track) = track.filter(|track| *track > 0) {
                summary.audio_tracks += tx.execute(
                    "INSERT OR IGNORE INTO media_audio_tracks(torrent_hash,track_id) VALUES(?1,?2)",
                    params![hash,track],
                ).map_err(|error| error.to_string())?;
            }
            if !title.trim().is_empty() {
                let poster_file = poster_file.filter(|name| valid_poster_name(name));
                let genres_json = serde_json::to_string(&parse_genres(&genres_json))
                    .map_err(|error| error.to_string())?;
                summary.metadata += tx.execute(
                    "INSERT OR IGNORE INTO media_metadata(torrent_hash,title,overview,year,rating,poster_file,genres_json)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![hash,title,overview,year,rating,poster_file,genres_json],
                ).map_err(|error| error.to_string())?;
                if genres_json != "[]" {
                    tx.execute(
                        "UPDATE media_metadata SET genres_json=?1 WHERE torrent_hash=?2 AND genres_json='[]'",
                        params![genres_json, hash],
                    ).map_err(|error| error.to_string())?;
                }
                if let Some(name) = poster_file {
                    posters_to_copy.push(name);
                }
            }
        }
        tx.commit().map_err(|error| error.to_string())?;
        if let (Some(destination), Some(source_parent)) = (poster_dir, source_path.parent()) {
            let source_posters = source_parent.join("cache").join("posters");
            for name in posters_to_copy {
                let source_file = source_posters.join(&name);
                let destination_file = destination.join(&name);
                if destination_file.is_file() || !source_file.is_file() {
                    continue;
                }
                let Ok(size) = std::fs::metadata(&source_file).map(|metadata| metadata.len())
                else {
                    continue;
                };
                if size == 0 || size > 8 * 1024 * 1024 {
                    continue;
                }
                std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
                std::fs::copy(source_file, destination_file).map_err(|error| error.to_string())?;
                summary.posters += 1;
            }
        }
        Ok(summary)
    }
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn snapshot_to(&self, destination: &Path) -> Result<(), String> {
        if destination.exists() {
            return Err(format!(
                "Файл копии уже существует: {}",
                destination.display()
            ));
        }
        let path = destination
            .to_str()
            .ok_or("Некорректный путь резервной копии")?;
        self.db
            .execute("VACUUM INTO ?1", [path])
            .map_err(|error| format!("Не удалось создать снимок SQLite: {error}"))?;
        Ok(())
    }

    fn from_connection(db: Connection) -> rusqlite::Result<Self> {
        db.busy_timeout(std::time::Duration::from_secs(3))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS media_file_history (
                torrent_hash TEXT NOT NULL,
                file_index INTEGER NOT NULL,
                file_name TEXT NOT NULL,
                file_path TEXT,
                first_played_at TEXT NOT NULL,
                playback_timecode INTEGER,
                playback_duration INTEGER,
                is_watched INTEGER NOT NULL DEFAULT 0,
                last_played_at TEXT NOT NULL,
                launch_count INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (torrent_hash,file_index)
            );
            CREATE TABLE IF NOT EXISTS media_types (
                torrent_hash TEXT PRIMARY KEY,
                media_type TEXT NOT NULL CHECK(media_type IN ('movie','series'))
            );
            CREATE TABLE IF NOT EXISTS media_audio_tracks (
                torrent_hash TEXT PRIMARY KEY,
                track_id INTEGER NOT NULL CHECK(track_id > 0)
            );
            CREATE TABLE IF NOT EXISTS media_subtitle_tracks (
                torrent_hash TEXT PRIMARY KEY,
                track_id INTEGER NOT NULL CHECK(track_id > 0)
            );
            CREATE TABLE IF NOT EXISTS media_web_audio_preferences (
                torrent_hash TEXT PRIMARY KEY,
                language TEXT NOT NULL,
                title TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS media_playback_preferences (
                torrent_hash TEXT PRIMARY KEY,
                season INTEGER,
                episode INTEGER,
                auto_next INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS media_metadata (
                torrent_hash TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                overview TEXT,
                year INTEGER,
                rating REAL,
                poster_file TEXT,
                genres_json TEXT NOT NULL DEFAULT '[]'
            );",
        )?;
        let has_genres: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('media_metadata') WHERE name='genres_json')",
            [], |row| row.get(0),
        )?;
        if !has_genres {
            db.execute(
                "ALTER TABLE media_metadata ADD COLUMN genres_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        Ok(Self { db })
    }

    pub fn mark_played(
        &self,
        hash: &str,
        file_index: i64,
        file_name: &str,
        file_path: Option<&str>,
    ) -> rusqlite::Result<()> {
        let hash = checked_key(hash, file_index)?;
        if file_name.trim().is_empty() {
            return Err(rusqlite::Error::InvalidParameterName("file_name".into()));
        }
        self.db.execute(
            "INSERT INTO media_file_history (
                torrent_hash,file_index,file_name,file_path,first_played_at,last_played_at,launch_count,is_watched
             ) VALUES (?1,?2,?3,?4,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'),1,1)
             ON CONFLICT(torrent_hash,file_index) DO UPDATE SET
                file_name=excluded.file_name,
                file_path=COALESCE(excluded.file_path,media_file_history.file_path),
                last_played_at=excluded.last_played_at,
                launch_count=media_file_history.launch_count+1,
                is_watched=1",
            params![hash, file_index, file_name, file_path],
        )?;
        Ok(())
    }

    pub fn save_progress(
        &self,
        hash: &str,
        file_index: i64,
        timecode: i64,
        duration: i64,
    ) -> rusqlite::Result<bool> {
        let hash = checked_key(hash, file_index)?;
        Ok(self.db.execute(
            "UPDATE media_file_history SET playback_timecode=?1,playback_duration=?2,
             last_played_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE torrent_hash=?3 AND file_index=?4",
            params![timecode.max(0), duration.max(0), hash, file_index],
        )? > 0)
    }

    pub fn set_viewed(&self, hash: &str, file_index: i64, viewed: bool) -> rusqlite::Result<bool> {
        let hash = checked_key(hash, file_index)?;
        Ok(self.db.execute(
            "UPDATE media_file_history SET is_watched=?1 WHERE torrent_hash=?2 AND file_index=?3",
            params![viewed, hash, file_index],
        )? > 0)
    }

    pub fn reset_position(&self, hash: &str, file_index: i64) -> rusqlite::Result<bool> {
        let hash = checked_key(hash, file_index)?;
        Ok(self.db.execute(
            "UPDATE media_file_history SET playback_timecode=0 WHERE torrent_hash=?1 AND file_index=?2",
            params![hash, file_index],
        )? > 0)
    }

    pub fn get(&self, hash: &str, file_index: i64) -> rusqlite::Result<Option<FileHistory>> {
        let hash = checked_key(hash, file_index)?;
        self.db.query_row(
            "SELECT file_index,file_name,file_path,playback_timecode,playback_duration,is_watched,launch_count
             FROM media_file_history WHERE torrent_hash=?1 AND file_index=?2",
            params![hash, file_index],
            |row| {
                Ok(FileHistory {
                    file_index: row.get(0)?,
                    file_name: row.get(1)?,
                    file_path: row.get(2)?,
                    playback_timecode: row.get(3)?,
                    playback_duration: row.get(4)?,
                    is_watched: row.get(5)?,
                    launch_count: row.get(6)?,
                })
            },
        ).optional()
    }

    pub fn recent(&self, limit: usize) -> rusqlite::Result<Vec<RecentPlayback>> {
        let mut statement = self.db.prepare(
            "SELECT torrent_hash,file_index,file_name,playback_timecode,playback_duration
             FROM media_file_history WHERE playback_timecode > 0
             AND (playback_duration IS NULL OR playback_duration = 0 OR playback_timecode < playback_duration - 60)
             ORDER BY last_played_at DESC LIMIT ?1",
        )?;
        let rows = statement
            .query_map([limit.min(50) as i64], |row| {
                Ok(RecentPlayback {
                    torrent_hash: row.get(0)?,
                    file_index: row.get(1)?,
                    file_name: row.get(2)?,
                    playback_timecode: row.get(3)?,
                    playback_duration: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                })
            })?
            .collect();
        rows
    }

    pub fn recently_played(&self, limit: usize) -> rusqlite::Result<Vec<RecentPlayback>> {
        let mut statement = self.db.prepare(
            "SELECT torrent_hash,file_index,file_name,playback_timecode,playback_duration
             FROM media_file_history ORDER BY last_played_at DESC LIMIT ?1",
        )?;
        let rows = statement
            .query_map([limit.min(50) as i64], |row| {
                Ok(RecentPlayback {
                    torrent_hash: row.get(0)?,
                    file_index: row.get(1)?,
                    file_name: row.get(2)?,
                    playback_timecode: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    playback_duration: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                })
            })?
            .collect();
        rows
    }

    pub fn media_type(&self, hash: &str) -> rusqlite::Result<Option<String>> {
        let hash = checked_key(hash, 0)?;
        self.db
            .query_row(
                "SELECT media_type FROM media_types WHERE torrent_hash=?1",
                [hash],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn metadata(&self, hash: &str) -> rusqlite::Result<Option<MediaMetadata>> {
        let hash = checked_key(hash, 0)?;
        self.db.query_row(
            "SELECT title,overview,year,rating,poster_file,genres_json FROM media_metadata WHERE torrent_hash=?1",
            [hash],
            |row| Ok(MediaMetadata {
                title: row.get(0)?,
                overview: row.get(1)?,
                year: row.get(2)?,
                rating: row.get(3)?,
                poster_file: row.get(4)?,
                genres: parse_genres(&row.get::<_, String>(5)?),
            }),
        ).optional()
    }

    pub fn set_title(&self, hash: &str, title: &str) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 200 {
            return Err(rusqlite::Error::InvalidParameterName("title".into()));
        }
        self.db.execute(
            "INSERT INTO media_metadata(torrent_hash,title) VALUES(?1,?2)
             ON CONFLICT(torrent_hash) DO UPDATE SET title=excluded.title",
            params![hash, title],
        )?;
        Ok(())
    }

    pub fn merge_remote_metadata(
        &self,
        hash: &str,
        original_title: &str,
        remote: &MediaMetadata,
    ) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        if remote.title.trim().is_empty()
            || remote.title.chars().count() > 200
            || remote
                .poster_file
                .as_deref()
                .is_some_and(|name| !valid_poster_name(name))
        {
            return Err(rusqlite::Error::InvalidParameterName("metadata".into()));
        }
        let current = self.metadata(&hash)?;
        let title = current
            .as_ref()
            .filter(|item| item.title != original_title)
            .map_or(remote.title.as_str(), |item| item.title.as_str());
        let previous_overview = current.as_ref().and_then(|item| item.overview.as_deref());
        let new_overview = remote
            .overview
            .as_deref()
            .filter(|text| !text.trim().is_empty());
        let has_cyrillic = |text: &str| {
            text.chars()
                .any(|ch| ('А'..='я').contains(&ch) || ch == 'ё' || ch == 'Ё')
        };
        let overview = match (previous_overview, new_overview) {
            (Some(previous), Some(new)) if has_cyrillic(previous) && !has_cyrillic(new) => {
                Some(previous)
            }
            (_, Some(new)) => Some(new),
            (previous, None) => previous,
        };
        let year = remote
            .year
            .or_else(|| current.as_ref().and_then(|item| item.year));
        let rating = remote
            .rating
            .or_else(|| current.as_ref().and_then(|item| item.rating));
        let poster = remote.poster_file.as_deref().or_else(|| {
            current
                .as_ref()
                .and_then(|item| item.poster_file.as_deref())
        });
        let genres = if remote.genres.is_empty() {
            current
                .as_ref()
                .map_or(&[][..], |item| item.genres.as_slice())
        } else {
            remote.genres.as_slice()
        };
        let genres_json = serde_json::to_string(genres)
            .map_err(|_| rusqlite::Error::InvalidParameterName("genres".into()))?;
        self.db.execute(
            "INSERT INTO media_metadata(torrent_hash,title,overview,year,rating,poster_file,genres_json)
             VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(torrent_hash) DO UPDATE SET
             title=excluded.title,overview=excluded.overview,year=excluded.year,
             rating=excluded.rating,poster_file=excluded.poster_file,genres_json=excluded.genres_json",
            params![hash, title, overview, year, rating, poster, genres_json],
        )?;
        Ok(())
    }

    pub fn has_viewed_file(&self, hash: &str) -> rusqlite::Result<bool> {
        let hash = checked_key(hash, 0)?;
        self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM media_file_history WHERE torrent_hash=?1 AND is_watched=1)",
            [hash],
            |row| row.get(0),
        )
    }

    pub fn set_media_type(&self, hash: &str, media_type: &str) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        if !matches!(media_type, "movie" | "series") {
            return Err(rusqlite::Error::InvalidParameterName("media_type".into()));
        }
        self.db.execute(
            "INSERT INTO media_types(torrent_hash,media_type) VALUES(?1,?2)
             ON CONFLICT(torrent_hash) DO UPDATE SET media_type=excluded.media_type",
            params![hash, media_type],
        )?;
        Ok(())
    }

    pub fn audio_track(&self, hash: &str) -> rusqlite::Result<Option<i64>> {
        let hash = checked_key(hash, 0)?;
        self.db
            .query_row(
                "SELECT track_id FROM media_audio_tracks WHERE torrent_hash=?1",
                [hash],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn web_audio_preference(&self, hash: &str) -> rusqlite::Result<Option<(String, String)>> {
        let hash = checked_key(hash, 0)?;
        self.db
            .query_row(
                "SELECT language,title FROM media_web_audio_preferences WHERE torrent_hash=?1",
                [hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
    }

    pub fn save_web_audio_preference(
        &self,
        hash: &str,
        language: &str,
        title: &str,
    ) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        if language.len() > 128 || title.len() > 512 {
            return Err(rusqlite::Error::InvalidParameterName(
                "audio preference".into(),
            ));
        }
        self.db.execute("INSERT INTO media_web_audio_preferences(torrent_hash,language,title) VALUES(?1,?2,?3) ON CONFLICT(torrent_hash) DO UPDATE SET language=excluded.language,title=excluded.title", params![hash,language,title])?;
        Ok(())
    }

    pub fn save_audio_track(&self, hash: &str, track_id: i64) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        if track_id < 1 {
            return Err(rusqlite::Error::InvalidParameterName("track_id".into()));
        }
        self.db.execute(
            "INSERT INTO media_audio_tracks(torrent_hash,track_id) VALUES(?1,?2)
             ON CONFLICT(torrent_hash) DO UPDATE SET track_id=excluded.track_id",
            params![hash, track_id],
        )?;
        Ok(())
    }

    pub fn subtitle_track(&self, hash: &str) -> rusqlite::Result<Option<i64>> {
        let hash = checked_key(hash, 0)?;
        self.db
            .query_row(
                "SELECT track_id FROM media_subtitle_tracks WHERE torrent_hash=?1",
                [hash],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_subtitle_track(&self, hash: &str, track_id: i64) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        if track_id < 1 {
            return Err(rusqlite::Error::InvalidParameterName("track_id".into()));
        }
        self.db.execute(
            "INSERT INTO media_subtitle_tracks(torrent_hash,track_id) VALUES(?1,?2)
             ON CONFLICT(torrent_hash) DO UPDATE SET track_id=excluded.track_id",
            params![hash, track_id],
        )?;
        Ok(())
    }

    pub fn playback_preferences(&self, hash: &str) -> rusqlite::Result<PlaybackPreferences> {
        let hash = checked_key(hash, 0)?;
        self.db
            .query_row(
                "SELECT season,episode,auto_next FROM media_playback_preferences WHERE torrent_hash=?1",
                [hash],
                |row| {
                    Ok(PlaybackPreferences {
                        season: row.get::<_, Option<i64>>(0)?.and_then(|value| u8::try_from(value).ok()),
                        episode: row.get::<_, Option<i64>>(1)?.and_then(|value| u16::try_from(value).ok()),
                        auto_next: row.get(2)?,
                    })
                },
            )
            .optional()
            .map(|value| value.unwrap_or_default())
    }

    pub fn save_playback_preferences(
        &self,
        hash: &str,
        preferences: &PlaybackPreferences,
    ) -> rusqlite::Result<()> {
        let hash = checked_key(hash, 0)?;
        self.db.execute(
            "INSERT INTO media_playback_preferences(torrent_hash,season,episode,auto_next)
             VALUES(?1,?2,?3,?4) ON CONFLICT(torrent_hash) DO UPDATE SET
             season=excluded.season,episode=excluded.episode,auto_next=excluded.auto_next",
            params![
                hash,
                preferences.season,
                preferences.episode,
                preferences.auto_next
            ],
        )?;
        Ok(())
    }
}

fn checked_key(hash: &str, file_index: i64) -> rusqlite::Result<String> {
    if file_index < 0 {
        return Err(rusqlite::Error::InvalidParameterName("file_index".into()));
    }
    normalize_info_hash(hash)
        .ok_or_else(|| rusqlite::Error::InvalidParameterName("torrent_hash".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_audio_identity_is_separate_from_mpv_track_numbers() {
        let db = HistoryStore::open_in_memory().unwrap();
        let hash = "0123456789abcdef0123456789abcdef01234567";
        db.save_audio_track(hash, 2).unwrap();
        db.save_web_audio_preference(hash, "rus", "Studio A")
            .unwrap();
        assert_eq!(
            db.web_audio_preference(hash).unwrap(),
            Some(("rus".into(), "Studio A".into()))
        );
        assert_eq!(db.audio_track(hash).unwrap(), Some(2));
        assert!(db
            .save_web_audio_preference(hash, "rus", &"x".repeat(513))
            .is_err());
        assert!(db
            .save_web_audio_preference("invalid", "rus", "Studio A")
            .is_err());
    }

    #[test]
    fn imports_electron_history_without_overwriting_rust_or_source() {
        assert!(!valid_poster_name("../secret.jpg"));
        assert!(!valid_poster_name("folder\\cover.jpg"));
        assert!(valid_poster_name("cinemeta-tt12345.jpg"));
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-electron-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("media.db");
        let source_posters = root.join("cache").join("posters");
        let rust_posters = root.join("rust-posters");
        std::fs::create_dir_all(&source_posters).unwrap();
        std::fs::write(source_posters.join("test.jpg"), b"poster").unwrap();
        let hash = "b".repeat(40);
        let source = Connection::open(&path).unwrap();
        source.execute_batch("CREATE TABLE media_file_history (
            torrent_hash TEXT,file_index INTEGER,file_name TEXT,file_path TEXT,first_played_at TEXT,
            playback_timecode INTEGER,playback_duration INTEGER,is_watched INTEGER,last_played_at TEXT,launch_count INTEGER);
            CREATE TABLE media_items (torrent_hash TEXT,media_type TEXT,audio_track_id INTEGER,title TEXT,overview TEXT,year INTEGER,rating REAL,poster_path TEXT,genres_json TEXT);").unwrap();
        source.execute("INSERT INTO media_file_history VALUES (?1,1,'Episode.mkv',NULL,'2026-01-01',300,1000,1,'2026-01-02',2)", [&hash]).unwrap();
        source.execute("INSERT INTO media_file_history VALUES (?1,2,'Episode 2.mkv',NULL,'2026-01-01',100,1000,1,'2026-01-03',1)", [&hash]).unwrap();
        source
            .execute("INSERT INTO media_items VALUES (?1,'series',2,'Русское название','Описание',2025,8.2,'test.jpg','[\"Драма\",\"Триллер\"]')", [&hash])
            .unwrap();
        drop(source);

        let mut rust = HistoryStore::open_in_memory().unwrap();
        rust.mark_played(&hash, 1, "Rust.mkv", None).unwrap();
        let first = rust.import_electron(&path, Some(&rust_posters)).unwrap();
        assert_eq!(first.files, 1);
        assert_eq!(first.media_types, 1);
        assert_eq!(first.audio_tracks, 1);
        assert_eq!(first.metadata, 1);
        assert_eq!(first.posters, 1);
        assert_eq!(
            std::fs::read(rust_posters.join("test.jpg")).unwrap(),
            b"poster"
        );
        assert_eq!(
            rust.metadata(&hash).unwrap().unwrap().title,
            "Русское название"
        );
        assert_eq!(
            rust.metadata(&hash).unwrap().unwrap().genres,
            ["Драма", "Триллер"]
        );
        assert_eq!(rust.get(&hash, 1).unwrap().unwrap().file_name, "Rust.mkv");
        assert_eq!(
            rust.get(&hash, 2).unwrap().unwrap().playback_timecode,
            Some(100)
        );
        rust.set_title(&hash, "Исправлено вручную").unwrap();
        rust.db
            .execute(
                "UPDATE media_metadata SET genres_json='[]' WHERE torrent_hash=?1",
                [&hash],
            )
            .unwrap();
        assert_eq!(
            rust.import_electron(&path, Some(&rust_posters))
                .unwrap()
                .posters,
            0
        );
        let after = rust.metadata(&hash).unwrap().unwrap();
        assert_eq!(after.title, "Исправлено вручную");
        assert_eq!(after.genres, ["Драма", "Триллер"]);
        assert_eq!(rust.audio_track(&hash).unwrap(), Some(2));
        let source = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let name: String = source
            .query_row(
                "SELECT file_name FROM media_file_history WHERE file_index=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Episode.mkv");
        drop(source);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(source_posters.join("test.jpg")).unwrap();
        std::fs::remove_file(rust_posters.join("test.jpg")).unwrap();
        std::fs::remove_dir(&rust_posters).unwrap();
        std::fs::remove_dir(&source_posters).unwrap();
        std::fs::remove_dir(root.join("cache")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn upgrades_existing_rust_metadata_table_without_losing_titles() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE media_metadata (torrent_hash TEXT PRIMARY KEY,title TEXT NOT NULL,overview TEXT,year INTEGER,rating REAL,poster_file TEXT);
            INSERT INTO media_metadata(torrent_hash,title) VALUES('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','Старое название');").unwrap();
        let history = HistoryStore::from_connection(db).unwrap();
        let metadata = history.metadata(&"a".repeat(40)).unwrap().unwrap();
        assert_eq!(metadata.title, "Старое название");
        assert!(metadata.genres.is_empty());
    }

    #[test]
    fn snapshot_is_complete_and_never_overwrites() {
        let path = std::env::temp_dir().join(format!(
            "pirate-cinema-rust-backup-test-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hash = "c".repeat(40);
        let history = HistoryStore::open_in_memory().unwrap();
        history
            .mark_played(&hash, 4, "Episode 4.mkv", None)
            .unwrap();
        history.save_progress(&hash, 4, 120, 600).unwrap();
        history.snapshot_to(&path).unwrap();
        assert!(history.snapshot_to(&path).is_err());
        let copy = HistoryStore::open(&path).unwrap();
        assert_eq!(
            copy.get(&hash, 4).unwrap().unwrap().playback_timecode,
            Some(120)
        );
        drop(copy);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn local_backup_contains_history_settings_and_manifest() {
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-rust-backup-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let history_path = root.join("source.sqlite");
        let settings_path = root.join("settings.json");
        let hash = "d".repeat(40);
        let db = HistoryStore::open(&history_path).unwrap();
        db.mark_played(&hash, 1, "Movie.mkv", None).unwrap();
        drop(db);
        std::fs::write(
            &settings_path,
            r#"{"torrserver_url":"http://127.0.0.1:8090","kinopoisk_api_key":"secret"}"#,
        )
        .unwrap();
        let poster_dir = root.join("cache").join("posters");
        std::fs::create_dir_all(&poster_dir).unwrap();
        std::fs::write(poster_dir.join("test.jpg"), b"cached poster").unwrap();
        let torrserver_dir = root.join("torrserver");
        std::fs::create_dir(&torrserver_dir).unwrap();
        std::fs::write(torrserver_dir.join("config.db"), b"torrserver database").unwrap();
        let backup = create_local_backup(
            &history_path,
            Some(&settings_path),
            Some(&torrserver_dir),
            &root.join("backups"),
        )
        .unwrap();
        let copied = HistoryStore::open(&backup.join("history.sqlite")).unwrap();
        assert!(copied.get(&hash, 1).unwrap().is_some());
        let copied_settings: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(backup.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(copied_settings["torrserver_url"], "http://127.0.0.1:8090");
        assert!(copied_settings.get("kinopoisk_api_key").is_none());
        assert!(backup.join("backup.json").is_file());
        assert_eq!(
            std::fs::read(backup.join("cache/posters/test.jpg")).unwrap(),
            b"cached poster"
        );
        assert_eq!(
            std::fs::read(backup.join("torrserver/config.db")).unwrap(),
            b"torrserver database"
        );
        drop(copied);
        for path in [
            backup.join("history.sqlite"),
            backup.join("settings.json"),
            backup.join("backup.json"),
            backup.join("cache/posters/test.jpg"),
            backup.join("torrserver/config.db"),
            poster_dir.join("test.jpg"),
            torrserver_dir.join("config.db"),
            history_path,
            settings_path,
        ] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(backup.join("cache/posters")).unwrap();
        std::fs::remove_dir(backup.join("cache")).unwrap();
        std::fs::remove_dir(backup.join("torrserver")).unwrap();
        std::fs::remove_dir(&torrserver_dir).unwrap();
        std::fs::remove_dir(&poster_dir).unwrap();
        std::fs::remove_dir(root.join("cache")).unwrap();
        std::fs::remove_dir(&backup).unwrap();
        std::fs::remove_dir(root.join("backups")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn restores_valid_rust_backup_over_current_state() {
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-rust-restore-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let history_path = root.join("history.sqlite");
        let settings_path = root.join("settings.json");
        let original_hash = "e".repeat(40);
        let later_hash = "f".repeat(40);
        let mut history = HistoryStore::open(&history_path).unwrap();
        history
            .mark_played(&original_hash, 1, "Original.mkv", None)
            .unwrap();
        std::fs::write(
            &settings_path,
            r#"{"torrserver_url":"http://127.0.0.1:8090"}"#,
        )
        .unwrap();
        let poster_dir = root.join("cache").join("posters");
        std::fs::create_dir_all(&poster_dir).unwrap();
        std::fs::write(poster_dir.join("original.jpg"), b"poster").unwrap();
        let torrserver_dir = root.join("torrserver");
        std::fs::create_dir(&torrserver_dir).unwrap();
        std::fs::write(torrserver_dir.join("config.db"), b"original torrents").unwrap();
        let backup = create_local_backup(
            &history_path,
            Some(&settings_path),
            Some(&torrserver_dir),
            &root,
        )
        .unwrap();

        history
            .mark_played(&later_hash, 2, "Later.mkv", None)
            .unwrap();
        std::fs::write(
            &settings_path,
            r#"{"torrserver_url":"http://localhost:9999"}"#,
        )
        .unwrap();
        std::fs::write(poster_dir.join("original.jpg"), b"changed").unwrap();
        std::fs::write(torrserver_dir.join("config.db"), b"changed torrents").unwrap();

        assert_eq!(
            history
                .restore_local_backup(&backup, Some(&settings_path), &root, Some(&torrserver_dir),)
                .unwrap(),
            1
        );
        assert!(history.get(&original_hash, 1).unwrap().is_some());
        assert!(history.get(&later_hash, 2).unwrap().is_none());
        assert!(std::fs::read_to_string(&settings_path)
            .unwrap()
            .contains("127.0.0.1:8090"));
        assert_eq!(
            std::fs::read(poster_dir.join("original.jpg")).unwrap(),
            b"poster"
        );
        assert_eq!(
            std::fs::read(torrserver_dir.join("config.db")).unwrap(),
            b"original torrents"
        );

        drop(history);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tracks_file_launch_and_position_independently_of_viewed_marker() {
        let db = HistoryStore::open_in_memory().unwrap();
        db.integrity_check().unwrap();
        let hash = "a".repeat(40);
        assert!(!db.has_viewed_file(&hash).unwrap());
        assert!(!db.save_progress(&hash, 1, 50, 100).unwrap());
        db.mark_played(&hash, 1, "Серия 1.mkv", None).unwrap();
        assert!(db.get(&hash, 1).unwrap().unwrap().is_watched);
        db.save_progress(&hash, 1, 50, 100).unwrap();
        db.set_viewed(&hash, 1, false).unwrap();
        db.save_progress(&hash, 1, 60, 100).unwrap();
        let file = db.get(&hash, 1).unwrap().unwrap();
        assert_eq!(file.playback_timecode, Some(60));
        assert!(!file.is_watched);
        db.reset_position(&hash, 1).unwrap();
        let file = db.get(&hash, 1).unwrap().unwrap();
        assert_eq!(file.playback_timecode, Some(0));
        assert_eq!(file.playback_duration, Some(100));
        assert!(!file.is_watched);
        db.mark_played(&hash, 1, "Серия 1.mkv", None).unwrap();
        assert_eq!(db.get(&hash, 1).unwrap().unwrap().launch_count, 2);
        assert!(db.get(&hash, 1).unwrap().unwrap().is_watched);
        assert!(db.get(&hash, 2).unwrap().is_none());
        db.save_subtitle_track(&hash, 4).unwrap();
        db.save_playback_preferences(
            &hash,
            &PlaybackPreferences {
                season: Some(3),
                episode: Some(8),
                auto_next: true,
            },
        )
        .unwrap();
        assert_eq!(db.subtitle_track(&hash).unwrap(), Some(4));
        assert_eq!(
            db.playback_preferences(&hash).unwrap(),
            PlaybackPreferences {
                season: Some(3),
                episode: Some(8),
                auto_next: true,
            }
        );
        assert_eq!(db.media_type(&hash).unwrap(), None);
        assert!(db.set_title(&hash, " ").is_err());
        db.set_title(&hash, "Русское название").unwrap();
        assert_eq!(
            db.metadata(&hash).unwrap().unwrap().title,
            "Русское название"
        );
        db.set_title(&hash, "Исправленное название").unwrap();
        assert_eq!(
            db.metadata(&hash).unwrap().unwrap().title,
            "Исправленное название"
        );
        db.merge_remote_metadata(
            &hash,
            "Original.Release.2024",
            &MediaMetadata {
                title: "Русское название из Википедии".into(),
                overview: Some("Описание на русском".into()),
                year: Some(2024),
                rating: None,
                poster_file: Some("poster.jpg".into()),
                genres: vec!["Драма".into()],
            },
        )
        .unwrap();
        let metadata = db.metadata(&hash).unwrap().unwrap();
        assert_eq!(metadata.title, "Исправленное название");
        assert_eq!(metadata.overview.as_deref(), Some("Описание на русском"));
        assert_eq!(metadata.poster_file.as_deref(), Some("poster.jpg"));
        assert_eq!(metadata.genres, vec!["Драма"]);
        db.merge_remote_metadata(
            &hash,
            "Original.Release.2024",
            &MediaMetadata {
                title: "English title".into(),
                overview: Some("English overview".into()),
                year: None,
                rating: None,
                poster_file: None,
                genres: Vec::new(),
            },
        )
        .unwrap();
        let metadata = db.metadata(&hash).unwrap().unwrap();
        assert_eq!(metadata.overview.as_deref(), Some("Описание на русском"));
        assert_eq!(metadata.poster_file.as_deref(), Some("poster.jpg"));
        db.set_media_type(&hash, "series").unwrap();
        assert_eq!(db.media_type(&hash).unwrap().as_deref(), Some("series"));
        assert!(db.set_media_type(&hash, "other").is_err());
        assert_eq!(db.audio_track(&hash).unwrap(), None);
        assert!(db.has_viewed_file(&hash).unwrap());
        db.save_audio_track(&hash, 2).unwrap();
        assert_eq!(db.audio_track(&hash).unwrap(), Some(2));
        assert!(db.save_audio_track(&hash, 0).is_err());
        db.save_progress(&hash, 1, 60, 1000).unwrap();
        assert_eq!(db.recent(10).unwrap().len(), 1);
        db.save_progress(&hash, 1, 995, 1000).unwrap();
        assert!(db.recent(10).unwrap().is_empty());
        assert_eq!(db.recently_played(10).unwrap().len(), 1);
    }
}
