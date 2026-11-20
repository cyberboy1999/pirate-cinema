//! Restricted network-rule subset for the native RuTube HLS loader.
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const SOURCES: [(&str,&str);2]=[
 ("uBlock filters – Ads","https://ublockorigin.github.io/uAssetsCDN/filters/filters.min.txt"),
 ("RU AdList","https://raw.githubusercontent.com/dimisa-RUAdList/RUAdListCDN/main/lists/ruadlist.ubo.min.txt")
];
const BASE: &str = "||rutube.ru/api/advert/\n||ac.rutube.ru/api/*/banner_rules?";
#[derive(Clone, Default, Serialize, Deserialize)]
struct Rules {
    updated: u64,
    sources: Vec<String>,
    block: Vec<String>,
    allow: Vec<String>,
    unsupported: usize,
}
static CACHE: LazyLock<Mutex<Option<Arc<Rules>>>> = LazyLock::new(|| Mutex::new(None));
fn path() -> Result<std::path::PathBuf, String> {
    Ok(crate::desktop_engine::profile_dir()?.join("adblock-network.json"))
}
fn compile(text: &str, rules: &mut Rules) {
    let mut conditional_depth = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("!#if ") {
            conditional_depth += 1;
            continue;
        }
        if line == "!#endif" {
            conditional_depth = conditional_depth.saturating_sub(1);
            continue;
        }
        if conditional_depth > 0 {
            if !line.starts_with('!') {
                rules.unsupported += 1;
            }
            continue;
        }

        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            continue;
        }
        let (allow, line) = line.strip_prefix("@@").map_or((false, line), |s| (true, s));
        // Conditional host-anchored exceptions are conservatively widened to
        // the whole host, never discarded while retaining its blocking rule.
        if allow && line.starts_with("||") && line.contains('$') {
            let host = line[2..]
                .split(['/', '^', '*', '?', '$'])
                .next()
                .unwrap_or_default();
            if host.contains('.')
                && host
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
            {
                rules.allow.push(format!("{host}^"));
                rules.unsupported += 1;
                continue;
            }
        }
        // Unsupported syntax is rejected rather than broadened into a blocking rule.
        let Some(pattern) = line.strip_prefix("||") else {
            rules.unsupported += 1;
            continue;
        };
        if (pattern.contains(['$', '#', '|'])
            || (pattern.contains('^') && !pattern.ends_with('^'))
            || (pattern.ends_with('^') && pattern[..pattern.len() - 1].contains(['/', '*', '?'])))
            || pattern.len() > 2048
        {
            rules.unsupported += 1;
            continue;
        }
        let host = pattern
            .split(['/', '^', '*', '?'])
            .next()
            .unwrap_or_default();
        if host.is_empty()
            || !host.contains('.')
            || !host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
        {
            rules.unsupported += 1;
            continue;
        }
        if allow {
            rules.allow.push(pattern.into());
        } else {
            rules.block.push(pattern.into());
        }
    }
}
#[cfg(test)]
fn matches(pattern: &str, url: &str) -> bool {
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    matches_uri(pattern, &uri)
}
fn matches_uri(pattern: &str, uri: &ureq::http::Uri) -> bool {
    let host = uri.host().unwrap_or_default().to_ascii_lowercase();
    let end = pattern.find(['/', '^', '*', '?']).unwrap_or(pattern.len());
    let domain = pattern[..end].to_ascii_lowercase();
    let tail = &pattern[end..];
    if host != domain && !host.ends_with(&format!(".{domain}")) {
        return false;
    }
    if tail.is_empty() || tail == "^" {
        return true;
    }
    // '^' with trailing text requires a full ABP engine; reject it in compile.
    let target = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let mut rest = target;
    let mut first = true;
    for part in tail.split('*') {
        if part.is_empty() {
            first = false;
            continue;
        }
        let Some(index) = rest.find(part) else {
            return false;
        };
        if first && index != 0 {
            return false;
        }
        rest = &rest[index + part.len()..];
        first = false;
    }
    true
}
fn load() -> Arc<Rules> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = cache.as_ref() {
        return r.clone();
    }
    let mut rules = path()
        .ok()
        .and_then(|p| {
            std::fs::File::open(p).ok().and_then(|file| {
                use std::io::Read;
                let mut bytes = Vec::new();
                file.take(16 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .ok()?;
                Some(bytes)
            })
        })
        .filter(|b| b.len() <= 16 * 1024 * 1024)
        .and_then(|b| serde_json::from_slice::<Rules>(&b).ok())
        .unwrap_or_default();
    compile(BASE, &mut rules);
    let rules = Arc::new(rules);
    *cache = Some(rules.clone());
    rules
}
pub fn blocked(url: &str) -> bool {
    let enabled = crate::desktop_engine::profile_dir()
        .ok()
        .and_then(|p| crate::settings::adblock_enabled(&p.join("settings.json")).ok())
        .unwrap_or(true);
    if !enabled {
        return false;
    }
    let rules = load();
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    !rules.allow.iter().any(|p| matches_uri(p, &uri))
        && rules.block.iter().any(|p| matches_uri(p, &uri))
}
pub fn status() -> String {
    let r = load();
    format!(
        "Сетевых правил: {}. Исключений: {}. Неподдерживаемых: {}. Обновление: {}",
        r.block.len(),
        r.allow.len(),
        r.unsupported,
        if r.updated == 0 {
            "встроенная база".into()
        } else {
            format!(
                "{} мин. назад",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .saturating_sub(r.updated)
                    / 60
            )
        }
    )
}
pub fn update() -> Result<String, String> {
    static UPDATE: Mutex<()> = Mutex::new(());
    let _guard = UPDATE
        .try_lock()
        .map_err(|_| "Обновление уже выполняется")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(2)
        .build()
        .into();
    let mut rules = Rules::default();
    for (name, url) in SOURCES {
        let text = agent
            .get(url)
            .call()
            .map_err(|e| format!("{name}: {e}"))?
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| format!("{name}: {e}"))?;
        if text.len() < 1000 || (!text.contains("! Title:") && !text.starts_with("[Adblock")) {
            return Err(format!(
                "{name}: некорректный список; сохранена предыдущая база"
            ));
        }
        compile(&text, &mut rules);
        rules.sources.push(url.into());
    }
    if rules.block.is_empty() {
        return Err("Нет поддерживаемых правил; предыдущая база сохранена".into());
    }
    rules.block.sort();
    rules.block.dedup();
    rules.allow.sort();
    rules.allow.dedup();
    rules.updated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let path = path()?;
    let temporary = path.with_extension("json.part");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&rules).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    replace(&temporary, &path)?;
    compile(BASE, &mut rules);
    *CACHE.lock().map_err(|_| "Кэш фильтров недоступен")? = Some(Arc::new(rules));
    Ok(status())
}
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(from, to).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_and_exceptions() {
        let mut r = Rules::default();
        compile("||rutube.ru/api/advert/\n@@||rutube.ru/api/advert/allowed\n||rutube.ru^$script\nrutube.ru##.ad",&mut r);
        assert_eq!(r.unsupported, 2);
        assert!(matches(&r.block[0], "https://rutube.ru/api/advert/vast"));
        assert!(!matches(
            &r.block[0],
            "https://rutube.ru.evil.test/api/advert/a"
        ));
        assert!(!matches(
            &r.block[0],
            "https://rutube.ru/video/a?next=/api/advert/"
        ));
        assert!(matches(&r.allow[0], "https://rutube.ru/api/advert/allowed"));
    }
    #[test]
    fn baseline() {
        let mut r = Rules::default();
        compile(BASE, &mut r);
        assert!(r
            .block
            .iter()
            .any(|p| matches(p, "https://ac.rutube.ru/api/v1/banner_rules?video=1")));
        assert!(!r
            .block
            .iter()
            .any(|p| matches(p, "https://bl.rutube.ru/video/segment.ts")));
    }
    #[test]
    fn conditional_rules_are_not_broadened() {
        let mut r = Rules::default();
        compile(
            "!#if env_firefox\n||rutube.ru^\n!#endif\n||ads.example^",
            &mut r,
        );
        assert_eq!(r.block, vec!["ads.example^"]);
        assert_eq!(r.unsupported, 1);
    }
    #[test]
    fn conditional_exception_preserves_playback() {
        let mut r = Rules::default();
        compile("||rutube.ru^\n@@||rutube.ru/content/$media", &mut r);
        assert!(r
            .allow
            .iter()
            .any(|p| matches(p, "https://rutube.ru/content/video.m3u8")));
    }
}
