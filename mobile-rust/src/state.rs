use pirate_cinema_core::{normalize_info_hash, VideoFile};
use std::io::Write;
use std::path::Path;

pub fn save_pending(profile: &Path, hash: &str, file: &VideoFile, url: &str) -> Result<(), String> {
    let hash = normalize_info_hash(hash).ok_or("Некорректная раздача")?;
    if file.id < 1 || file.name.is_empty() {
        return Err("Некорректный файл".into());
    }
    std::fs::create_dir_all(profile).map_err(|e| e.to_string())?;
    let value = serde_json::json!({"hash":hash,"id":file.id,"name":file.name,"path":file.path,"length":file.length,"url":url});
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(profile.join("pending-player.json"))
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "Дождитесь возврата из плеера".to_owned()
            } else {
                e.to_string()
            }
        })?;
    output
        .write_all(value.to_string().as_bytes())
        .map_err(|e| e.to_string())
}

pub fn load_pending(profile: &Path) -> Result<Option<(String, VideoFile, String)>, String> {
    let path = profile.join("pending-player.json");
    if !path.exists() {
        return Ok(None);
    }
    if path.metadata().map_err(|e| e.to_string())?.len() > 16384 {
        return Err("Некорректное состояние плеера".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let hash = value["hash"]
        .as_str()
        .and_then(normalize_info_hash)
        .ok_or("Некорректная сохранённая раздача")?;
    let id = value["id"]
        .as_i64()
        .filter(|id| *id > 0)
        .ok_or("Некорректный индекс")?;
    let name = value["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .ok_or("Нет названия файла")?
        .to_owned();
    let path = value["path"].as_str().unwrap_or(&name).to_owned();
    Ok(Some((
        hash,
        VideoFile {
            id,
            name,
            path,
            length: value["length"].as_u64().unwrap_or(0),
        },
        value["url"].as_str().unwrap_or_default().to_owned(),
    )))
}

pub fn clear_pending(profile: &Path) -> Result<(), String> {
    match std::fs::remove_file(profile.join("pending-player.json")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn matches_result(expected: &str, actual: Option<&str>) -> bool {
    // Legacy local prototypes had no URL marker and allowed only one launch.
    expected.is_empty() || actual == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_late_results_after_abandoning_a_session() {
        let previous = "http://127.0.0.1:12345/torrents/test/stream/0?pc_session=1";
        let current = "http://127.0.0.1:12345/torrents/test/stream/0?pc_session=2";
        assert!(!matches_result(current, Some(previous)));
        assert!(!matches_result(current, None));
        assert!(matches_result(current, Some(current)));
    }
    #[test]
    fn restores_launch_identity_without_process_memory() {
        let directory =
            std::env::temp_dir().join(format!("pc-mobile-pending-{}", std::process::id()));
        let file = VideoFile {
            id: 7,
            name: "Серия.mkv".into(),
            path: "Season/Серия.mkv".into(),
            length: 42,
        };
        let hash = "a".repeat(40);
        let url = "http://127.0.0.1:12345/torrents/test/stream/0?pc_session=test";
        save_pending(&directory, &hash, &file, url).unwrap();
        let mut other_file = file.clone();
        other_file.id = 8;
        assert!(save_pending(&directory, &hash, &other_file, url).is_err());
        assert_eq!(
            load_pending(&directory).unwrap(),
            Some((hash, file, url.into()))
        );
        clear_pending(&directory).unwrap();
        assert_eq!(load_pending(&directory).unwrap(), None);
        std::fs::remove_dir(&directory).unwrap();
    }
}
