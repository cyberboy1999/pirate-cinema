use crate::DEFAULT_TORRSERVER_URL;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerType {
    BundledMpv,
    External,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub language: Language,
    pub onboarding_complete: bool,
    pub torrserver_url: String,
    pub player_type: PlayerType,
    pub player_path: String,
    pub embedded_player: bool,
    pub torznab_url: String,
    pub torznab_api_key: String,
}

pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    let value = read_settings(path)?;
    let endpoint = value
        .get("torrserver_url")
        .and_then(|value| value.as_str())
        .unwrap_or(DEFAULT_TORRSERVER_URL);
    Ok(Preferences {
        language: match value.get("language").and_then(|value| value.as_str()) {
            Some("en") => Language::English,
            _ => Language::Russian,
        },
        onboarding_complete: value
            .get("onboarding_complete")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        torrserver_url: validate_endpoint(endpoint)?,
        player_type: match value.get("player_type").and_then(|value| value.as_str()) {
            Some("external") => PlayerType::External,
            _ => PlayerType::BundledMpv,
        },
        player_path: value
            .get("player_path")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned(),
        embedded_player: value
            .get("embedded_player")
            .and_then(|value| value.as_bool())
            .unwrap_or(cfg!(windows)),
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
    })
}

pub fn save_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    let endpoint = validate_endpoint(&preferences.torrserver_url)?;
    validate_player(preferences.player_type, &preferences.player_path)?;
    validate_torznab(&preferences.torznab_url, &preferences.torznab_api_key)?;
    let mut value = read_settings(path)?;
    value["language"] = serde_json::json!(match preferences.language {
        Language::Russian => "ru",
        Language::English => "en",
    });
    value["onboarding_complete"] = serde_json::json!(preferences.onboarding_complete);
    value["torrserver_url"] = serde_json::json!(endpoint);
    value["player_type"] = serde_json::json!(if preferences.player_type == PlayerType::External {
        "external"
    } else {
        "mpv"
    });
    value["player_path"] = serde_json::json!(preferences.player_path.trim());
    value["embedded_player"] = serde_json::json!(preferences.embedded_player);
    value["torznab_url"] = serde_json::json!(preferences.torznab_url.trim());
    value["torznab_api_key"] = serde_json::json!(preferences.torznab_api_key.trim());
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

pub fn load_player(path: &Path) -> Result<(PlayerType, String), String> {
    if !path.exists() {
        return Ok((PlayerType::BundledMpv, String::new()));
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let kind = match value.get("player_type").and_then(|value| value.as_str()) {
        Some("external") => PlayerType::External,
        _ => PlayerType::BundledMpv,
    };
    let path = value
        .get("player_path")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    Ok((kind, path.to_owned()))
}

pub fn validate_player(kind: PlayerType, path: &str) -> Result<(), String> {
    if kind == PlayerType::External {
        let executable = Path::new(path.trim());
        if !executable.is_file() {
            return Err("Укажите существующий файл локального плеера".into());
        }
        #[cfg(target_os = "windows")]
        if !executable
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Err("Укажите существующий .exe файл локального плеера".into());
        }
    }
    Ok(())
}

pub fn save_player(path: &Path, kind: PlayerType, player_path: &str) -> Result<(), String> {
    validate_player(kind, player_path)?;
    let mut value = if path.exists() {
        serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
    } else {
        serde_json::json!({"torrserver_url": DEFAULT_TORRSERVER_URL})
    };
    value["player_type"] = serde_json::json!(if kind == PlayerType::External {
        "external"
    } else {
        "mpv"
    });
    value["player_path"] = serde_json::json!(player_path.trim());
    let parent = path
        .parent()
        .ok_or("Не удалось определить папку настроек")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value.to_string()).map_err(|error| error.to_string())
}

pub fn load_endpoint(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Ok(DEFAULT_TORRSERVER_URL.into());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let endpoint = value
        .get("torrserver_url")
        .and_then(|value| value.as_str())
        .ok_or("В настройках нет адреса TorrServer")?;
    validate_endpoint(endpoint)
}

pub fn save_endpoint(path: &Path, endpoint: &str) -> Result<(), String> {
    let endpoint = validate_endpoint(endpoint)?;
    let mut value = if path.exists() {
        serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
    } else {
        serde_json::json!({})
    };
    value["torrserver_url"] = serde_json::json!(endpoint);
    let parent = path
        .parent()
        .ok_or("Не удалось определить папку настроек")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value.to_string()).map_err(|error| error.to_string())
}

pub fn remove_retired_provider_settings(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let removed = value
        .as_object_mut()
        .is_some_and(|settings| settings.remove("kinopoisk_api_key").is_some());
    if removed {
        std::fs::write(path, value.to_string()).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn validate_endpoint(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim().trim_end_matches('/').trim();
    let url = ureq::http::Uri::try_from(endpoint).map_err(|_| "Некорректный адрес TorrServer")?;
    if !matches!(url.scheme_str(), Some("http" | "https")) || url.authority().is_none() {
        return Err("Адрес TorrServer должен начинаться с http:// или https://".into());
    }
    if url.path() != "" && url.path() != "/" || url.query().is_some() {
        return Err("Укажите только базовый адрес TorrServer, без пути и параметров".into());
    }
    Ok(endpoint.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_base_http_endpoint() {
        assert_eq!(
            validate_endpoint(" http://127.0.0.1:8090/ ").unwrap(),
            DEFAULT_TORRSERVER_URL
        );
        assert!(validate_endpoint("file:///tmp").is_err());
        assert!(validate_endpoint("http://localhost:8090/torrents").is_err());
    }

    #[test]
    fn rejects_missing_external_player() {
        assert!(validate_player(PlayerType::External, "C:\\missing\\player.exe").is_err());
        assert!(validate_player(PlayerType::BundledMpv, "").is_ok());
    }

    #[test]
    fn removes_retired_provider_token_without_touching_other_settings() {
        let path = std::env::temp_dir().join(format!(
            "pirate-cinema-retired-provider-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"torrserver_url":"http://127.0.0.1:8090","kinopoisk_api_key":"secret"}"#,
        )
        .unwrap();
        remove_retired_provider_settings(&path).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["torrserver_url"], DEFAULT_TORRSERVER_URL);
        assert!(value.get("kinopoisk_api_key").is_none());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn saves_first_run_and_torznab_preferences_together() {
        let path = std::env::temp_dir().join(format!(
            "pirate-cinema-preferences-{}.json",
            std::process::id()
        ));
        let preferences = Preferences {
            language: Language::English,
            onboarding_complete: true,
            torrserver_url: DEFAULT_TORRSERVER_URL.into(),
            player_type: PlayerType::BundledMpv,
            player_path: String::new(),
            embedded_player: true,
            torznab_url: "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/".into(),
            torznab_api_key: "secret".into(),
        };
        save_preferences(&path, &preferences).unwrap();
        let loaded = load_preferences(&path).unwrap();
        assert_eq!(loaded.language, preferences.language);
        assert!(loaded.onboarding_complete);
        assert_eq!(loaded.torrserver_url, preferences.torrserver_url);
        assert_eq!(loaded.player_type, preferences.player_type);
        assert_eq!(loaded.torznab_api_key, "secret");
        assert_eq!(
            loaded.torznab_url,
            "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn requires_complete_torznab_credentials() {
        assert!(validate_torznab("http://127.0.0.1:9117/api", "").is_err());
        assert!(validate_torznab("", "secret").is_err());
        assert!(validate_torznab("", "").is_ok());
    }
}
