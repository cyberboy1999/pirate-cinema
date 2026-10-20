//! Android's external-player boundary. This module never owns TorrServe or MPV processes.
#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
use crate::{settings::validate_media_endpoint, stream_url, VideoFile};
use serde::{Deserialize, Serialize};

#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
pub const TORRSERVE_PACKAGE: &str = "ru.yourok.torrserve";
pub const MPV_PACKAGE: &str = "is.xyz.mpv";

/// Called only after an explicit Search action. A stock Android server disables
/// its local search index by default; preserve every unrelated server setting.
#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
pub fn prepare_search(endpoint: &str) -> Result<bool, String> {
    let endpoint = format!("{}/settings", validate_media_endpoint(endpoint)?);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    let mut settings: serde_json::Value = agent
        .post(&endpoint)
        .send_json(serde_json::json!({"action":"get"}))
        .map_err(|e| format!("Настройки поиска TorrServe: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    if !enable_search_flag(&mut settings)? {
        return Ok(false);
    }
    agent
        .post(endpoint)
        .send_json(serde_json::json!({"action":"set","sets":settings}))
        .map_err(|e| format!("Не удалось включить поиск TorrServe: {e}"))?;
    Ok(true)
}

#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
fn enable_search_flag(settings: &mut serde_json::Value) -> Result<bool, String> {
    let object = settings
        .as_object_mut()
        .ok_or("Некорректные настройки TorrServe")?;
    if object
        .get("EnableRutorSearch")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        return Ok(false);
    }
    object.insert("EnableRutorSearch".into(), serde_json::json!(true));
    Ok(true)
}

/// Values for the native Android ACTION_VIEW bridge, not a shell command.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PlaybackRequest {
    pub package: &'static str,
    pub url: String,
    pub title: String,
    pub position_ms: i32,
}

#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
pub fn playback_request(
    endpoint: &str,
    hash: &str,
    file: &VideoFile,
    title: &str,
    position_ms: i64,
) -> Result<PlaybackRequest, String> {
    let endpoint = validate_media_endpoint(endpoint)?;
    if title.trim().is_empty() || title.contains('\0') {
        return Err("Укажите название видео".into());
    }
    let position_ms = i32::try_from(position_ms)
        .map_err(|_| "Позиция не поддерживается внешним Android-плеером")?;
    if position_ms < 0 {
        return Err("Позиция не может быть отрицательной".into());
    }
    Ok(PlaybackRequest {
        package: MPV_PACKAGE,
        url: stream_url(&endpoint, hash, file)?,
        title: title.trim().to_owned(),
        position_ms,
    })
}

/// Missing result data is not evidence that playback finished.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PlaybackResult {
    pub source_url: Option<String>,
    pub position_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub completed: bool,
}

impl PlaybackResult {
    /// SQLite desktop history uses seconds; Android player extras use milliseconds.
    /// Unknown/incomplete results must not overwrite previously saved progress.
    pub fn progress_seconds(&self) -> Result<Option<(i64, i64)>, String> {
        let (Some(position), Some(duration)) = (self.position_ms, self.duration_ms) else {
            return Ok(None);
        };
        if position < 0 || duration <= 0 || position > duration {
            return Err("Плеер вернул некорректную позицию".into());
        }
        Ok(Some((position / 1000, duration / 1000)))
    }
}

/// Probe both known local ports while keeping a configured custom endpoint first.
#[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
pub fn local_server_candidates(current: &str) -> Result<Vec<String>, String> {
    let mut candidates = vec![validate_media_endpoint(current)?];
    for endpoint in ["http://127.0.0.1:8091", "http://127.0.0.1:8090"] {
        if !candidates.iter().any(|value| value == endpoint) {
            candidates.push(endpoint.to_owned());
        }
    }
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
    #[test]
    fn enables_local_search_without_resetting_server_settings() {
        let mut settings = serde_json::json!({"EnableRutorSearch":false,"CacheSize":123,"EnableTorznabSearch":true,"TorznabUrls":[{"Host":"http://localhost:9117","Key":"test"}]});
        let original = settings.clone();
        assert!(enable_search_flag(&mut settings).unwrap());
        assert_eq!(settings["CacheSize"], original["CacheSize"]);
        assert_eq!(settings["TorznabUrls"], original["TorznabUrls"]);
        assert_eq!(
            settings["EnableTorznabSearch"],
            original["EnableTorznabSearch"]
        );
        assert!(!enable_search_flag(&mut settings).unwrap());
        assert!(enable_search_flag(&mut serde_json::Value::Null).is_err());
    }

    #[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
    fn file() -> VideoFile {
        VideoFile {
            id: 7,
            name: "Серия 2.mkv".into(),
            path: "S01/Серия 2.mkv".into(),
            length: 42,
        }
    }

    #[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
    #[test]
    fn creates_external_mpv_request_without_gst_or_id_rewriting() {
        let request = playback_request(
            "http://127.0.0.1:8090/",
            &"a".repeat(40),
            &file(),
            " Шоу ",
            125_000,
        )
        .unwrap();
        assert_eq!(request.package, MPV_PACKAGE);
        assert_eq!(request.title, "Шоу");
        assert_eq!(request.position_ms, 125_000);
        assert!(request.url.ends_with("index=7&play"));
        assert!(request.url.starts_with("http://127.0.0.1:8090/stream/%D0"));
        assert!(!request.url.contains("/gst/"));
    }

    #[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
    #[test]
    fn rejects_unsafe_or_invalid_launch_values() {
        for endpoint in [
            "file:///data/local",
            "http://localhost:8090/stream",
            "http://localhost:8090?x=1",
        ] {
            assert!(playback_request(endpoint, &"a".repeat(40), &file(), "Movie", 0).is_err());
        }
        for position in [-1, i64::MAX] {
            assert!(playback_request(
                "http://localhost:8090",
                &"a".repeat(40),
                &file(),
                "Movie",
                position
            )
            .is_err());
        }
        assert!(playback_request("http://localhost:8090", "bad", &file(), "Movie", 0).is_err());
    }

    #[test]
    fn preserves_unknown_progress_and_checks_android_units() {
        assert_eq!(PlaybackResult::default().progress_seconds().unwrap(), None);
        let empty: PlaybackResult = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.progress_seconds().unwrap(), None);
        assert!(!empty.completed);
        let value: PlaybackResult = serde_json::from_str(
            r#"{"position_ms":125900,"duration_ms":1500000,"completed":false}"#,
        )
        .unwrap();
        assert_eq!(value.progress_seconds().unwrap(), Some((125, 1500)));
        assert!(!value.completed);
        assert!(PlaybackResult {
            source_url: None,
            position_ms: Some(20),
            duration_ms: Some(10),
            completed: false
        }
        .progress_seconds()
        .is_err());
    }

    #[cfg(all(not(target_os = "android"), not(feature = "torrent-engine")))]
    #[test]
    fn keeps_custom_server_and_supports_both_local_ports() {
        assert_eq!(
            local_server_candidates("http://192.168.1.10:9000").unwrap(),
            [
                "http://192.168.1.10:9000",
                "http://127.0.0.1:8091",
                "http://127.0.0.1:8090"
            ]
        );
        assert_eq!(
            local_server_candidates("http://127.0.0.1:8090")
                .unwrap()
                .len(),
            2
        );
    }
}
