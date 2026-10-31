use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Russian,
    English,
}

impl Language {
    pub fn pick<'a>(self, russian: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::Russian => russian,
            Self::English => english,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoverMode {
    /// Clean generated art when metadata is present, cached poster otherwise.
    #[default]
    Auto,
    /// Always build a deterministic local cover from the best available title.
    Generated,
    /// Prefer the cached provider poster, using generated art only as a fallback.
    Poster,
}

impl CoverMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Generated => "generated",
            Self::Poster => "poster",
        }
    }

    pub fn from_setting(value: &str) -> Self {
        match value {
            "generated" => Self::Generated,
            "poster" => Self::Poster,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub language: Language,
    pub onboarding_complete: bool,
    pub torznab_url: String,
    pub torznab_api_key: String,
    pub close_to_tray: bool,
    pub cover_mode: CoverMode,
}

pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    let value = read_settings(path)?;
    Ok(Preferences {
        language: match value.get("language").and_then(|value| value.as_str()) {
            Some("en") => Language::English,
            _ => Language::Russian,
        },
        onboarding_complete: value
            .get("onboarding_complete")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        torznab_url: value
            .get("torznab_url")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned(),
        torznab_api_key: value
            .get("torznab_api_key")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned(),
        close_to_tray: value
            .get("close_to_tray")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        cover_mode: value
            .get("cover_mode")
            .and_then(|value| value.as_str())
            .map(CoverMode::from_setting)
            .unwrap_or_default(),
    })
}

pub fn save_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    validate_torznab(&preferences.torznab_url, &preferences.torznab_api_key)?;
    let mut value = read_settings(path)?;
    value["language"] = serde_json::json!(match preferences.language {
        Language::Russian => "ru",
        Language::English => "en",
    });
    value["onboarding_complete"] = serde_json::json!(preferences.onboarding_complete);
    if let Some(settings) = value.as_object_mut() {
        for retired in [
            "torrserver_url",
            "player_type",
            "player_path",
            "embedded_player",
            "kinopoisk_api_key",
        ] {
            settings.remove(retired);
        }
    }
    value["torznab_url"] = serde_json::json!(preferences.torznab_url.trim());
    value["torznab_api_key"] = serde_json::json!(preferences.torznab_api_key.trim());
    value["close_to_tray"] = serde_json::json!(preferences.close_to_tray);
    value["cover_mode"] = serde_json::json!(preferences.cover_mode.as_str());
    write_settings(path, &value)
}

/// Persist only the cover choice so changing it never overwrites unsaved
/// Torznab fields in the settings form.
pub fn save_cover_mode(path: &Path, mode: CoverMode) -> Result<(), String> {
    let mut value = read_settings(path)?;
    value["cover_mode"] = serde_json::json!(mode.as_str());
    write_settings(path, &value)
}

fn read_settings(path: &Path) -> Result<serde_json::Value, String> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn write_settings(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Не удалось определить папку настроек")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value.to_string()).map_err(|error| error.to_string())
}

pub fn validate_torznab(url: &str, api_key: &str) -> Result<(), String> {
    let url = url.trim();
    let api_key = api_key.trim();
    if url.is_empty() && api_key.is_empty() {
        return Ok(());
    }
    if url.is_empty() || api_key.is_empty() {
        return Err("Для Jackett/Torznab нужны URL и API-ключ".into());
    }
    let uri = ureq::http::Uri::try_from(url).map_err(|_| "Некорректный URL Jackett/Torznab")?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err("URL Jackett/Torznab должен начинаться с http:// или https://".into());
    }
    Ok(())
}

pub fn validate_media_endpoint(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim().trim_end_matches('/').trim();
    let url = ureq::http::Uri::try_from(endpoint).map_err(|_| "Некорректный адрес медиасервера")?;
    if !matches!(url.scheme_str(), Some("http" | "https")) || url.authority().is_none() {
        return Err("Адрес медиасервера должен начинаться с http:// или https://".into());
    }
    if url.path() != "" && url.path() != "/" || url.query().is_some() {
        return Err("Укажите только базовый адрес медиасервера, без пути и параметров".into());
    }
    Ok(endpoint.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_first_run_and_torznab_preferences_together() {
        let path = std::env::temp_dir().join(format!(
            "pirate-cinema-preferences-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"torrserver_url":"http://127.0.0.1:8090","player_type":"mpv","kinopoisk_api_key":"retired"}"#,
        )
        .unwrap();
        let preferences = Preferences {
            language: Language::English,
            onboarding_complete: true,
            torznab_url: "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/".into(),
            torznab_api_key: "secret".into(),
            close_to_tray: false,
            cover_mode: CoverMode::Generated,
        };
        save_preferences(&path, &preferences).unwrap();
        let loaded = load_preferences(&path).unwrap();
        assert_eq!(loaded.language, preferences.language);
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.torznab_api_key, "secret");
        assert!(!loaded.close_to_tray);
        assert_eq!(loaded.cover_mode, CoverMode::Generated);
        assert_eq!(
            loaded.torznab_url,
            "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/"
        );
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for retired in ["torrserver_url", "player_type", "kinopoisk_api_key"] {
            assert!(
                saved.get(retired).is_none(),
                "retired key {retired} remains"
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn requires_complete_torznab_credentials() {
        assert!(validate_torznab("http://127.0.0.1:9117/api", "").is_err());
        assert!(validate_torznab("", "secret").is_err());
        assert!(validate_torznab("", "").is_ok());
    }

    #[test]
    fn cover_mode_is_independent_and_defaults_to_auto() {
        let path = std::env::temp_dir().join(format!(
            "pirate-cinema-cover-mode-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"language":"en","torznab_url":"http://127.0.0.1:9117/api","torznab_api_key":"secret"}"#,
        )
        .unwrap();
        assert_eq!(load_preferences(&path).unwrap().cover_mode, CoverMode::Auto);
        save_cover_mode(&path, CoverMode::Poster).unwrap();
        let loaded = load_preferences(&path).unwrap();
        assert_eq!(loaded.cover_mode, CoverMode::Poster);
        assert_eq!(loaded.torznab_api_key, "secret");
        assert!(std::fs::remove_file(path).is_ok());
    }
}
