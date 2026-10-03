use crate::history::{HistoryStore, ImportSummary};
use std::path::{Path, PathBuf};

const TORRSERVER_FILES: &[&str] = &[
    "config.db",
    "settings.json",
    "viewed.json",
    "trackers.txt",
    "rutor.ls",
];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrationSummary {
    pub electron: Option<ImportSummary>,
    pub prototype_history: bool,
    pub settings: bool,
    pub torrserver_files: usize,
}

pub fn migrate_legacy_profiles(
    destination: &Path,
    electron: Option<&Path>,
    prototype: Option<&Path>,
) -> Result<MigrationSummary, String> {
    let marker = destination.join("migration-0.5.8-complete.json");
    if marker.is_file() {
        return Ok(MigrationSummary::default());
    }
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut summary = MigrationSummary::default();
    let history_path = destination.join("history.sqlite");
    if !history_path.exists() {
        if let Some(source) = prototype
            .map(|root| root.join("history.sqlite"))
            .filter(|path| path.is_file())
        {
            std::fs::copy(source, &history_path).map_err(|error| error.to_string())?;
            summary.prototype_history = true;
        }
    }
    let poster_dir = destination.join("cache").join("posters");
    copy_tree_files(
        prototype.map(|root| root.join("cache").join("posters")),
        &poster_dir,
    )?;
    if let Some(database) = electron
        .map(|root| root.join("data").join("media.db"))
        .filter(|path| path.is_file())
    {
        let mut history = HistoryStore::open(&history_path).map_err(|error| error.to_string())?;
        summary.electron = Some(history.import_electron(&database, Some(&poster_dir))?);
    } else if !history_path.exists() {
        HistoryStore::open(&history_path).map_err(|error| error.to_string())?;
    }
    let settings_path = destination.join("settings.json");
    if !settings_path.exists() {
        if let Some(source) = prototype
            .map(|root| root.join("settings.json"))
            .filter(|path| path.is_file())
        {
            std::fs::copy(source, &settings_path).map_err(|error| error.to_string())?;
            summary.settings = true;
        } else if let Some(source) = electron
            .map(|root| root.join("data").join("config.json"))
            .filter(|path| path.is_file())
        {
            migrate_settings(&source, &settings_path)?;
            summary.settings = true;
        }
    }
    let torrserver = destination.join("torrserver");
    std::fs::create_dir_all(&torrserver).map_err(|error| error.to_string())?;
    for root in [prototype, electron].into_iter().flatten() {
        let source = root.join("torrserver");
        for name in TORRSERVER_FILES {
            let from = source.join(name);
            let to = torrserver.join(name);
            if !to.exists() && from.is_file() {
                std::fs::copy(from, to).map_err(|error| error.to_string())?;
                summary.torrserver_files += 1;
            }
        }
    }
    let temporary = marker.with_extension("json.part");
    std::fs::write(
        &temporary,
        serde_json::to_vec_pretty(&serde_json::json!({
            "source": "Electron 0.5.8 and Rust prototype",
            "historyImported": summary.electron.is_some(),
            "prototypeHistoryCopied": summary.prototype_history,
            "settingsMigrated": summary.settings,
            "torrserverFilesCopied": summary.torrserver_files,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::rename(temporary, marker).map_err(|error| error.to_string())?;
    Ok(summary)
}

pub fn legacy_profile_paths() -> (Option<PathBuf>, Option<PathBuf>) {
    let electron_override = std::env::var_os("PIRATE_CINEMA_ELECTRON_PROFILE").map(PathBuf::from);
    let prototype_override = std::env::var_os("PIRATE_CINEMA_PROTOTYPE_PROFILE").map(PathBuf::from);
    #[cfg(windows)]
    {
        let electron = electron_override.or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|root| root.join("pirate-cinema-desktop"))
        });
        let prototype = prototype_override.or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|root| root.join("Pirate Cinema Rust"))
        });
        (electron, prototype)
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let electron = electron_override.or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home.as_ref().map(|root| root.join(".config")))
                .map(|root| root.join("pirate-cinema-desktop"))
        });
        let prototype = prototype_override.or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| home.map(|root| root.join(".local/share")))
                .map(|root| root.join("Pirate Cinema Rust"))
        });
        (electron, prototype)
    }
}

fn migrate_settings(source: &Path, destination: &Path) -> Result<(), String> {
    let legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let settings = serde_json::json!({
        "language": legacy.get("language").and_then(|value| value.as_str()).unwrap_or("ru"),
        "onboarding_complete": legacy.get("onboardingComplete").and_then(|value| value.as_bool()).unwrap_or(true),
        "torrserver_url": legacy.get("torrServerUrl").and_then(|value| value.as_str()).unwrap_or(crate::DEFAULT_TORRSERVER_URL),
        "player_type": legacy.get("playerType").and_then(|value| value.as_str()).unwrap_or("mpv"),
        "player_path": legacy.get("playerPath").and_then(|value| value.as_str()).unwrap_or(""),
        "torznab_url": legacy.get("torznabUrl").and_then(|value| value.as_str()).unwrap_or(""),
        "torznab_api_key": legacy.get("torznabApiKey").and_then(|value| value.as_str()).unwrap_or(""),
    });
    std::fs::write(
        destination,
        serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn copy_tree_files(source: Option<PathBuf>, destination: &Path) -> Result<(), String> {
    let Some(source) = source.filter(|path| path.is_dir()) else {
        return Ok(());
    };
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
            && !target.exists()
        {
            std::fs::copy(entry.path(), target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrates_electron_once_without_changing_the_source() {
        let root =
            std::env::temp_dir().join(format!("pirate-cinema-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let electron = root.join("electron");
        let destination = root.join("destination");
        std::fs::create_dir_all(electron.join("data/cache/posters")).unwrap();
        std::fs::create_dir_all(electron.join("torrserver")).unwrap();
        std::fs::write(
            electron.join("data/config.json"),
            r#"{"language":"en","onboardingComplete":true,"playerType":"mpv"}"#,
        )
        .unwrap();
        std::fs::write(electron.join("torrserver/config.db"), b"database").unwrap();
        let database = electron.join("data/media.db");
        let db = Connection::open(&database).unwrap();
        db.execute_batch("CREATE TABLE media_file_history(torrent_hash TEXT,file_index INTEGER,file_name TEXT,file_path TEXT,first_played_at TEXT,playback_timecode INTEGER,playback_duration INTEGER,is_watched INTEGER,last_played_at TEXT,launch_count INTEGER);CREATE TABLE media_items(torrent_hash TEXT,media_type TEXT,audio_track_id INTEGER,title TEXT,overview TEXT,year INTEGER,rating REAL,poster_path TEXT,genres_json TEXT);").unwrap();
        drop(db);
        let first = migrate_legacy_profiles(&destination, Some(&electron), None).unwrap();
        assert!(first.electron.is_some());
        assert!(first.settings);
        assert_eq!(first.torrserver_files, 1);
        assert_eq!(
            std::fs::read(electron.join("torrserver/config.db")).unwrap(),
            b"database"
        );
        assert_eq!(
            migrate_legacy_profiles(&destination, Some(&electron), None).unwrap(),
            MigrationSummary::default()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
