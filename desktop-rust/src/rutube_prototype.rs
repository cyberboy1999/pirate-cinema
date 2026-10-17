use base64::Engine;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const PLAYER: &str = include_str!("../assets/rutube-player.js");
const CSS: &str = r#"
.rutube-section{color:#eee;padding:20px 0}.rutube-section header{display:flex;align-items:center;justify-content:space-between}.rutube-section h1{font-size:27px}.rutube-section small{color:#aaa}.rutube-section form{display:flex;gap:12px;margin:24px 0}.rutube-section input{flex:1;min-width:0;background:#141414;color:white;border:1px solid #333;padding:14px;border-radius:10px;font:inherit}.rutube-section button{font:inherit;cursor:pointer;border:1px solid #333;border-radius:9px;padding:10px 18px;background:#eee;color:#111}.rutube-section button:disabled{opacity:.5;cursor:wait}.rutube-section button:focus-visible,.rutube-section a:focus-visible{outline:2px solid #398bff}.rutube-section a{color:#8abaff}.rutube-section .results{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:20px}.rutube-section .card{text-align:left;background:#141414;color:#eee;padding:0;overflow:hidden}.rutube-section .card img{width:100%;aspect-ratio:16/9;object-fit:cover;display:block}.rutube-section .card strong,.rutube-section .card small{display:block;margin:12px;overflow-wrap:anywhere;white-space:normal}.rutube-section .notice{padding:14px;border:1px solid #333;border-radius:10px;margin:15px 0}#rutube-player{width:100%;aspect-ratio:16/9;background:#111;border-radius:12px;overflow:hidden}.rutube-section .hidden{display:none}.rutube-section .secondary{background:#181818;color:#ddd}.rutube-section nav{display:flex;gap:12px;margin:20px 0}
.rutube-section.is-playing{padding-top:0}
.rutube-section.is-playing > header,.rutube-section.is-playing > form,.rutube-section.is-playing > nav,.rutube-section.is-playing > .results{display:none}
.content:has(.rutube-section.is-playing) > .topbar,.content:has(.rutube-section.is-playing) > .notice{display:none}
"#;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Video {
    id: String,
    title: String,
    #[serde(default)]
    thumbnail_url: String,
    #[serde(default)]
    duration: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct Watched {
    video: Video,
    playlist: String,
    position: f64,
    duration: f64,
}

fn history_db() -> Result<rusqlite::Connection, String> {
    let data = pirate_cinema_core::torrserver_process::default_data_dir()?;
    let root = data.parent().ok_or("Не найдена папка профиля")?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let db = rusqlite::Connection::open(root.join("rutube-history.sqlite"))
        .map_err(|e| e.to_string())?;
    init_history(&db).map_err(|e| e.to_string())?;
    Ok(db)
}

fn init_history(db: &rusqlite::Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS watched (id TEXT PRIMARY KEY, video TEXT NOT NULL, playlist TEXT NOT NULL, position REAL NOT NULL, duration REAL NOT NULL, updated INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS online_preferences (key TEXT PRIMARY KEY, value INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS playlists (url TEXT PRIMARY KEY, title TEXT NOT NULL);")
}

fn set_preference(db: &rusqlite::Connection, key: &str, value: i64) -> Result<(), String> {
    if !matches!((key, value), ("quality", 0..=4320) | ("auto_next", 0..=1)) {
        return Err("Некорректная настройка".into());
    }
    db.execute("INSERT INTO online_preferences VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",rusqlite::params![key,value]).map_err(|e|e.to_string())?;
    Ok(())
}

fn preference(key: &str, fallback: i64) -> i64 {
    history_db()
        .ok()
        .and_then(|db| {
            db.query_row(
                "SELECT value FROM online_preferences WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .ok()
        })
        .unwrap_or(fallback)
}

fn saved_playlists() -> Result<Vec<(String, String)>, String> {
    let db = history_db()?;
    let mut query = db
        .prepare("SELECT url,title FROM playlists ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;
    let rows = query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn bookmark(url: &str, title: &str, remove: bool) -> Result<(), String> {
    edit_bookmark(&history_db()?, url, title, remove)
}

fn edit_bookmark(
    db: &rusqlite::Connection,
    url: &str,
    title: &str,
    remove: bool,
) -> Result<(), String> {
    if playlist_id(url).is_none() {
        return Err("Нужна ссылка на плейлист Rutube".into());
    }
    if remove { db.execute("DELETE FROM playlists WHERE url=?1",[url]) } else {
        db.execute("INSERT INTO playlists VALUES (?1,?2) ON CONFLICT(url) DO UPDATE SET title=excluded.title",rusqlite::params![url,title])
    }.map_err(|e|e.to_string())?;
    Ok(())
}

fn save_history(db: &rusqlite::Connection, item: &Watched) -> Result<(), String> {
    if !valid_id(&item.video.id)
        || !item.position.is_finite()
        || !item.duration.is_finite()
        || item.position < 0.0
        || item.duration <= 0.0
        || item.position > item.duration + 1.0
    {
        return Err("Некорректная позиция Rutube".into());
    }
    let video = serde_json::to_string(&item.video).map_err(|e| e.to_string())?;
    db.execute("INSERT INTO watched VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET video=excluded.video,playlist=excluded.playlist,position=excluded.position,duration=excluded.duration,updated=excluded.updated",
        rusqlite::params![item.video.id, video, item.playlist, item.position, item.duration,
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_history(db: &rusqlite::Connection) -> Result<Vec<Watched>, String> {
    let mut statement = db
        .prepare(
            "SELECT video,playlist,position,duration FROM watched ORDER BY updated DESC,rowid DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        let (video, playlist, position, duration) = row.map_err(|e| e.to_string())?;
        if let Ok(video) = serde_json::from_str::<Video>(&video) {
            items.push(Watched {
                video,
                playlist,
                position,
                duration,
            });
        }
    }
    Ok(items)
}

fn resume_position(items: &[Watched], id: &str) -> f64 {
    items
        .iter()
        .find(|item| item.video.id == id)
        .filter(|item| item.position + 5.0 < item.duration)
        .map_or(0.0, |item| item.position)
}

fn change_history(
    db: &rusqlite::Connection,
    mut item: Watched,
    action: &str,
) -> Result<Vec<Watched>, String> {
    match action {
        "remove" => {
            db.execute("DELETE FROM watched WHERE id=?1", [&item.video.id])
                .map_err(|e| e.to_string())?;
        }
        "reset" | "viewed" => {
            item.position = if action == "viewed" {
                item.duration
            } else {
                0.0
            };
            save_history(db, &item)?;
        }
        _ => return Err("Неизвестное действие истории".into()),
    }
    read_history(db)
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn request(path: &str, query: Option<(&str, &str)>) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut request = agent.get(format!("https://rutube.ru/api/{path}"));
    if let Some((key, value)) = query {
        request = request.query(key, value);
    }
    request
        .call()
        .map_err(|_| "Rutube не отвечает или ограничил доступ. Попробуйте позднее.".to_string())?
        .body_mut()
        .read_json()
        .map_err(|_| "Некорректный ответ Rutube".to_string())
}

fn search(query: &str, page: u32) -> Result<(Vec<Video>, bool), String> {
    if query.trim().is_empty() || query.chars().count() > 200 {
        return Err("Введите запрос длиной от 1 до 200 символов".into());
    }
    if let Some(id) = playlist_id(query) {
        let mut items = Vec::new();
        for page in 1..=25 {
            let (videos, next) = parse_videos(request(
                &format!("playlist/custom/{id}/videos?page={page}&format=json"),
                None,
            )?)?;
            for video in videos {
                if !items.iter().any(|item: &Video| item.id == video.id) {
                    items.push(video);
                }
            }
            if !next {
                return Ok((items, false));
            }
        }
        return Err(
            "Плейлист слишком большой: поддерживается до 25 страниц. Откройте его на Rutube."
                .into(),
        );
    }
    let value = request(
        &format!("search/video/?format=json&page={page}"),
        Some(("query", query)),
    )?;
    parse_videos(value)
}

fn playlist_id(text: &str) -> Option<&str> {
    let uri: ureq::http::Uri = text.trim().parse().ok()?;
    if uri.scheme_str() != Some("https") || uri.authority()?.as_str() != "rutube.ru" {
        return None;
    }
    let path = text
        .trim()
        .split('?')
        .next()?
        .strip_prefix("https://rutube.ru/plst/")?;
    let id = path.trim_end_matches('/');
    (!id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

fn parse_videos(value: Value) -> Result<(Vec<Video>, bool), String> {
    let results = value["results"]
        .as_array()
        .ok_or("Нет результатов в ответе Rutube")?;
    let videos = results
        .iter()
        .filter(|v| {
            !["is_deleted", "is_hidden", "is_paid", "is_locked"]
                .iter()
                .any(|key| v[*key] == true)
        })
        .filter_map(|v| serde_json::from_value::<Video>(v.clone()).ok())
        .filter(|v| valid_id(&v.id))
        .take(100)
        .collect();
    Ok((videos, value["has_next"].as_bool().unwrap_or(false)))
}

fn hls_from_options(value: &Value) -> Result<String, String> {
    if value["acl_access"]["allowed"] == false || value["is_paid"] == true {
        return Err("Rutube ограничил доступ к этому ролику".into());
    }
    let url = value["video_balancer"]["m3u8"]
        .as_str()
        .ok_or("Открытый HLS недоступен. Посмотрите ролик на Rutube.")?;
    validate_media_url(url)?;
    Ok(url.into())
}

fn validate_media_url(url: &str) -> Result<(), String> {
    let parsed: ureq::http::Uri = url.parse().map_err(|_| "Некорректный адрес HLS")?;
    let host = parsed.host().unwrap_or_default();
    if parsed.scheme_str() != Some("https")
        || parsed.authority().is_some_and(|a| a.as_str().contains('@'))
        || !["rutube.ru", "rtbcdn.ru"]
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    {
        return Err("Неизвестный адрес видеопотока Rutube".into());
    }
    if parsed.port_u16().is_some_and(|port| port != 443) {
        return Err("Недопустимый порт CDN".into());
    }
    Ok(())
}

fn fetch_media(url: &str, range: Option<(u64, u64)>) -> Result<String, String> {
    validate_media_url(url)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .max_redirects(0)
        .build()
        .into();
    let mut request = agent.get(url);
    if let Some((start, end)) = range {
        if end <= start || end - start > 32 * 1024 * 1024 {
            return Err("Некорректный диапазон HLS".into());
        }
        request = request.header("Range", format!("bytes={start}-{}", end - 1));
    }
    let mut response = request.call().map_err(|_| "CDN Rutube не ответил")?;
    if range.is_some() && response.status().as_u16() != 206 {
        return Err("CDN не поддержал диапазон HLS".into());
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_vec()
        .map_err(|_| "Не удалось прочитать фрагмент Rutube")?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn resolve(id: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err("Некорректный ID видео".into());
    }
    hls_from_options(&request(&format!("play/options/{id}/?format=json"), None)?)
}

#[component]
pub fn RutubePage(
    active: ReadSignal<bool>,
    #[props(default)] home: ReadSignal<bool>,
    #[props(default)] on_open: EventHandler<()>,
) -> Element {
    let desktop = dioxus::desktop::use_window();
    let mut query = use_signal(String::new);
    let mut active_query = use_signal(String::new);
    let mut page = use_signal(|| 1u32);
    let mut videos = use_signal(Vec::<Video>::new);
    let mut has_next = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut message = use_signal(|| {
        "Введите запрос. В Rutube отправляется только текст поиска; медиатека не используется."
            .to_string()
    });
    let mut selected = use_signal(|| None::<Video>);
    let mut generation = use_signal(|| 0u64);
    let mut watched = use_signal(|| {
        history_db()
            .and_then(|db| read_history(&db))
            .unwrap_or_default()
    });
    let mut bookmarks = use_signal(|| saved_playlists().unwrap_or_default());
    let close_window = desktop.clone();
    use_effect(move || {
        let _ = home();
        if !active() {
            let next = *generation.peek() + 1;
            generation.set(next);
            document::eval("window.rutubeFlush?.(); window.rutubePlayer?.destroy(false); window.rutubePlayer=null;");
            close_window.set_fullscreen(false);
            selected.set(None);
        }
    });
    let mut do_search = move |target: u32, text: String| {
        if busy() {
            return;
        }
        busy.set(true);
        message.set("Поиск на Rutube…".into());
        spawn(async move {
            let search_text = text.clone();
            let result = tokio::task::spawn_blocking(move || search(&search_text, target)).await;
            match result {
                Ok(Ok((items, next))) => {
                    message.set(format!("Найдено на странице: {}", items.len()));
                    videos.set(items);
                    has_next.set(next);
                    page.set(target);
                    active_query.set(text);
                }
                Ok(Err(error)) => message.set(error),
                Err(_) => message.set("Ошибка задачи поиска".into()),
            }
            busy.set(false);
        });
    };
    let close_button_window = desktop.clone();
    let play = move |video: Video| {
        if busy() {
            return;
        }
        busy.set(true);
        message.set("Получаем HLS Rutube…".into());
        let token = generation() + 1;
        generation.set(token);
        let player_window = desktop.clone();
        spawn(async move {
            let id = video.id.clone();
            let result = tokio::task::spawn_blocking(move || resolve(&id)).await;
            if generation() != token {
                busy.set(false);
                return;
            }
            match result {
                Ok(Ok(url)) => {
                    selected.set(Some(video.clone()));
                    let queue = videos();
                    let playlist = if playlist_id(&active_query()).is_some() {
                        active_query()
                    } else {
                        String::new()
                    };
                    let config = serde_json::json!({"url":url,"title":video.title,"token":token,
                        "queue":if !playlist.is_empty() {queue.clone()} else {vec![]}, "id":video.id,
                        "position":resume_position(&watched(), &video.id),"quality":preference("quality",0),"autoNext":preference("auto_next",1)!=0});
                    let script = format!(
                        "{}\n{}\n{}\nwindow.startRutube({});",
                        include_str!("../assets/hls.min.js"),
                        include_str!("../assets/artplayer.js"),
                        PLAYER,
                        config
                    );
                    let mut eval = document::eval(&script);
                    message.set("Загрузка видео…".into());
                    busy.set(false);
                    while let Ok(event) = eval.recv::<Value>().await {
                        if let (Some(id), Some(position), Some(duration)) = (
                            event["progress"].as_str(),
                            event["position"].as_f64(),
                            event["duration"].as_f64(),
                        ) {
                            if let Some(item) = queue
                                .iter()
                                .chain(std::iter::once(&video))
                                .find(|item| item.id == id)
                            {
                                let entry = Watched {
                                    video: item.clone(),
                                    playlist: playlist.clone(),
                                    position,
                                    duration,
                                };
                                match history_db().and_then(|db| {
                                    save_history(&db, &entry)?;
                                    read_history(&db)
                                }) {
                                    Ok(items) => watched.set(items),
                                    Err(error) => {
                                        message.set(format!("История Rutube не сохранена: {error}"))
                                    }
                                }
                            }
                        }
                        if generation() != token {
                            break;
                        }
                        if event["closed"] == true {
                            player_window.set_fullscreen(false);
                            break;
                        }
                        if let Some(fullscreen) = event["fullscreen"].as_bool() {
                            player_window.set_fullscreen(fullscreen);
                        }
                        if let Some(status) = event["status"].as_str() {
                            message.set(status.into());
                        }
                        if let (Some(key), Some(value)) =
                            (event["preference"].as_str(), event["value"].as_i64())
                        {
                            if let Err(error) =
                                history_db().and_then(|db| set_preference(&db, key, value))
                            {
                                message.set(error);
                            }
                        }
                        if let Some(id) = event["retry"].as_str() {
                            if selected.peek().as_ref().is_some_and(|item| item.id == id) {
                                let id = id.to_owned();
                                let result =
                                    tokio::task::spawn_blocking(move || resolve(&id)).await;
                                if generation() != token {
                                    break;
                                }
                                match result {
                                    Ok(Ok(url)) => {
                                        let args = serde_json::json!(url);
                                        document::eval(&format!("window.rutubeRecover?.({args});"));
                                    }
                                    _ => {
                                        message.set("Не удалось восстановить поток. Нажмите «Повторить» в плеере.".into());
                                        document::eval("window.rutubeRetryFailed?.();");
                                    }
                                }
                            }
                        }
                        if let Some(id) = event["episode"].as_str() {
                            if let Some(next) = queue.iter().find(|item| item.id == id).cloned() {
                                let next_id = next.id.clone();
                                message.set("Загрузка следующего выпуска…".into());
                                let result =
                                    tokio::task::spawn_blocking(move || resolve(&next_id)).await;
                                if generation() != token {
                                    break;
                                }
                                match result {
                                    Ok(Ok(url)) => {
                                        selected.set(Some(next.clone()));
                                        let args = serde_json::json!({"url":url,"id":next.id,"position":resume_position(&watched(), &next.id)});
                                        document::eval(&format!("window.rutubeSwitch?.({args});"));
                                    }
                                    _ => {
                                        message.set(
                                            "Выпуск недоступен. Выберите другой в меню плеера."
                                                .into(),
                                        );
                                        document::eval("window.rutubeSwitchFailed?.();");
                                    }
                                }
                            }
                        }
                        if let (Some(id), Some(url)) = (event["id"].as_u64(), event["url"].as_str())
                        {
                            let url = url.to_string();
                            let range = event["start"].as_u64().zip(event["end"].as_u64());
                            spawn(async move {
                                let result =
                                    tokio::task::spawn_blocking(move || fetch_media(&url, range))
                                        .await;
                                let (data, error) = match result {
                                    Ok(Ok(data)) => (data, String::new()),
                                    Ok(Err(error)) => (String::new(), error),
                                    Err(_) => (String::new(), "Ошибка загрузки фрагмента".into()),
                                };
                                if generation() == token {
                                    let args = serde_json::json!([token, id, data, error]);
                                    document::eval(&format!("window.rutubeReply?.(...{args});"));
                                }
                            });
                        }
                    }
                    return;
                }
                Ok(Err(error)) => message.set(error),
                Err(_) => message.set("Ошибка загрузки видео".into()),
            }
            busy.set(false);
        });
    };
    let resume_play = play.clone();
    let resume = move |item: Watched| {
        let mut start = resume_play.clone();
        let token = generation();
        spawn(async move {
            if !item.playlist.is_empty() {
                busy.set(true);
                let url = item.playlist.clone();
                let result = tokio::task::spawn_blocking(move || search(&url, 1)).await;
                if generation() != token {
                    busy.set(false);
                    return;
                }
                match result {
                    Ok(Ok((items, _))) => {
                        videos.set(items);
                        active_query.set(item.playlist.clone());
                        query.set(item.playlist);
                    }
                    _ => {
                        message.set("Не удалось восстановить плейлист. Попробуйте ещё раз.".into());
                        busy.set(false);
                        return;
                    }
                }
                busy.set(false);
            } else {
                videos.set(vec![item.video.clone()]);
                active_query.set(String::new());
            }
            page.set(1);
            has_next.set(false);
            on_open.call(());
            start(item.video);
        });
    };
    let history_action = move |(item, action): (Watched, &'static str)| {
        let result = history_db().and_then(|db| change_history(&db, item, action));
        match result {
            Ok(items) => {
                watched.set(items);
                true
            }
            Err(error) => {
                message.set(error);
                false
            }
        }
    };
    let mut seen = std::collections::HashSet::new();
    let recent = watched()
        .into_iter()
        .filter(|item| {
            seen.insert(if item.playlist.is_empty() {
                item.video.id.clone()
            } else {
                item.playlist.clone()
            })
        })
        .take(12)
        .collect::<Vec<_>>();
    rsx! {
        style { "{CSS}" }
        if !recent.is_empty() && (home() || (active() && selected().is_none())) {
            section { class: "rutube-section",
                if !recent.is_empty() { h2 { "Недавно смотрели онлайн" } }
                div { class: "results",
                    for item in recent {
                        button { class: "card", key: "{item.video.id}", disabled: busy(), onclick: { let item = item.clone(); let resume = resume.clone(); move |_| resume(item.clone()) },
                            img { src: "{item.video.thumbnail_url}", alt: "", loading: "lazy", referrerpolicy: "no-referrer" }
                            strong { "{item.video.title}" }
                            small { "{item.position as u64 / 60} / {item.duration as u64 / 60} мин · Продолжить" }
                        }
                    }
                }
            }
        }
        div { style: if home() { "display:none" } else { "display:block" },
        div { class: if selected().is_some() { "rutube-section is-playing" } else { "rutube-section" },
            header { h1 { "Онлайн-видео" } small { "Прямое воспроизведение · без TorrServer" } }
            form { onsubmit: move |event| { event.prevent_default(); do_search(1, query()); },
                input { aria_label: "Поиск видео или ссылка на плейлист Rutube", placeholder: "Поиск или https://rutube.ru/plst/…", maxlength: 200, value: "{query}", oninput: move |event| query.set(event.value()) }
                button { r#type: "submit", disabled: busy(), "Найти" }
            }
            div { class: "notice", role: "status", "{message}" }
            if selected().is_none() {
                nav {
                    button { class:"secondary", disabled: busy() || playlist_id(&active_query()).is_none(), onclick: move |_| {
                        let title=videos.peek().first().map(|v|v.title.clone()).unwrap_or_else(&*active_query);
                        match bookmark(&active_query(),&title,false).and_then(|_|saved_playlists()) { Ok(items)=>bookmarks.set(items),Err(error)=>message.set(error) }
                    }, "Сохранить плейлист" }
                }
                details { summary { "Сохранённые плейлисты ({bookmarks().len()})" }
                    for (url,title) in bookmarks() {
                        nav {
                            button { class:"secondary", disabled:busy(), onclick:{ let url=url.clone(); move |_| {query.set(url.clone()); do_search(1,url.clone());} }, "{title}" }
                            button { class:"secondary", disabled:busy(), onclick:move |_| { match bookmark(&url,"",true).and_then(|_|saved_playlists()) {Ok(items)=>bookmarks.set(items),Err(error)=>message.set(error)} }, "Убрать из сохранённых" }
                        }
                    }
                }
                details { summary { "История просмотра ({watched().len()})" }
                    for item in watched().into_iter().take(100) {
                        div { class:"notice", strong { "{item.video.title}" }
                            small { if item.position + 5.0 >= item.duration { " · Просмотрено" } else { " · {item.position as u64 / 60} / {item.duration as u64 / 60} мин" } }
                            nav {
                                button { class:"secondary",disabled:busy(),onclick:{let item=item.clone();let resume=resume.clone();move |_|resume(item.clone())},"Продолжить" }
                                button { class:"secondary",disabled:busy(),onclick:{let item=item.clone();let mut action=history_action;let resume=resume.clone();move |_|{if action((item.clone(),"reset")){resume(item.clone());}}},"С начала" }
                                button { class:"secondary",disabled:busy(),onclick:{let item=item.clone();let mut action=history_action;move |_|{action((item.clone(),"viewed"));}},"Просмотрено" }
                                button { class:"secondary",disabled:busy(),onclick:{let item=item.clone();let mut action=history_action;move |_|{action((item.clone(),"remove"));}},"Удалить запись" }
                            }
                        }
                    }
                }
            }
            div { id: "rutube-watch-space", class: if selected().is_some() { "" } else { "hidden" },
                if let Some(video) = selected() {
                    h2 { "{video.title}" }
                    nav {
                        button { class: "secondary", onclick: { let window = close_button_window.clone(); move |_| {
                            generation.set(generation() + 1);
                            document::eval("window.rutubeFlush?.(); window.rutubePlayer?.destroy(false); window.rutubePlayer=null; document.querySelector('.rutube-section')?.scrollIntoView({block:'start'});");
                            window.set_fullscreen(false);
                            selected.set(None);
                        } }, "Назад к поиску" }
                        a { href: "https://rutube.ru/video/{video.id}/", target: "_blank", rel: "noopener noreferrer", "Открыть на Rutube" }
                    }
                }
                div { id: "rutube-player" }
            }
            nav {
                button { class: "secondary", disabled: busy() || page() <= 1, onclick: move |_| do_search(page() - 1, active_query()), "Назад" }
                span { "Страница {page}" }
                button { class: "secondary", disabled: busy() || !has_next(), onclick: move |_| do_search(page() + 1, active_query()), "Далее" }
            }
            div { class: "results",
                for video in videos() {
                    button { class: "card", key: "{video.id}", disabled: busy(), onclick: { let item = video.clone(); let mut start = play.clone(); move |_| start(item.clone()) },
                        img { src: "{video.thumbnail_url}", alt: "", loading: "lazy", referrerpolicy: "no-referrer" }
                        strong { "{video.title}" }
                        small { "{video.duration / 60} мин · Rutube" }
                    }
                }
            }
        }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_survives_reopen_and_keeps_each_episode() {
        let path = std::env::temp_dir().join(format!(
            "pirate-rutube-history-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db = rusqlite::Connection::open(&path).unwrap();
        init_history(&db).unwrap();
        let mut item = Watched {
            video: Video {
                id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                title: "Первая".into(),
                thumbnail_url: String::new(),
                duration: 120,
            },
            playlist: "https://rutube.ru/plst/359607/".into(),
            position: 42.0,
            duration: 120.0,
        };
        save_history(&db, &item).unwrap();
        set_preference(&db, "quality", 720).unwrap();
        set_preference(&db, "auto_next", 0).unwrap();
        edit_bookmark(&db, "https://rutube.ru/plst/359607/", "Сериал", false).unwrap();
        edit_bookmark(
            &db,
            "https://rutube.ru/plst/359607/",
            "Новое название",
            false,
        )
        .unwrap();
        assert!(edit_bookmark(&db, "https://invalid.test/plst/1/", "", false).is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM playlists", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        edit_bookmark(&db, "https://rutube.ru/plst/359607/", "", true).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM playlists", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        edit_bookmark(&db, "https://rutube.ru/plst/359607/", "Сериал", false).unwrap();
        assert!(set_preference(&db, "unknown", 1).is_err());
        assert!(set_preference(&db, "quality", -1).is_err());
        assert_eq!(
            change_history(&db, item.clone(), "viewed").unwrap()[0].position,
            120.0
        );
        assert_eq!(
            change_history(&db, item.clone(), "reset").unwrap()[0].position,
            0.0
        );
        assert!(change_history(&db, item.clone(), "remove")
            .unwrap()
            .is_empty());
        save_history(&db, &item).unwrap();
        item.video.id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        item.position = 119.0;
        save_history(&db, &item).unwrap();
        item.position = f64::NAN;
        assert!(save_history(&db, &item).is_err());
        drop(db);
        let db = rusqlite::Connection::open(&path).unwrap();
        let items = read_history(&db).unwrap();
        assert_eq!(
            db.query_row("SELECT title FROM playlists", [], |row| row
                .get::<_, String>(0))
                .unwrap(),
            "Сериал"
        );
        assert_eq!(
            db.query_row(
                "SELECT value FROM online_preferences WHERE key='quality'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            720
        );
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].video.id, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert_eq!(
            resume_position(&items, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            42.0
        );
        assert_eq!(
            resume_position(&items, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            0.0
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    #[ignore = "contacts a public Rutube playlist"]
    fn live_rutube_playlist() {
        let (items, next) = search("https://rutube.ru/plst/973044/", 1).unwrap();
        assert!(!next);
        assert_eq!(items[0].id, "f1184b075ace35ffc7cb484d6b672f1c");
    }
    #[test]
    fn playlist_keeps_provider_order_and_skips_unavailable() {
        let (items, next) = parse_videos(serde_json::json!({"has_next":true,"results":[
            {"id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","title":"2"},
            {"id":"cccccccccccccccccccccccccccccccc","title":"hidden","is_hidden":true},
            {"id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","title":"1"}
        ]}))
        .unwrap();
        assert!(next);
        assert_eq!(
            items.iter().map(|v| v.title.as_str()).collect::<Vec<_>>(),
            vec!["2", "1"]
        );
    }
    #[test]
    #[ignore = "contacts public Rutube and downloads one fragment of an open video"]
    fn live_rutube_fragment() {
        let mut url = resolve("9906a8ef5c0a1f7dbcbc08e81ac63845").unwrap();
        for _ in 0..3 {
            let data = fetch_media(&url, None).unwrap();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .unwrap();
            if !bytes.starts_with(b"#EXTM3U") {
                assert!(bytes.len() > 1024);
                return;
            }
            let text = String::from_utf8(bytes).unwrap();
            let next = text
                .lines()
                .find(|line| !line.is_empty() && !line.starts_with('#'))
                .unwrap();
            url = if next.starts_with("https://") {
                next.to_string()
            } else if next.starts_with('/') {
                let uri: ureq::http::Uri = url.parse().unwrap();
                format!("https://{}{next}", uri.authority().unwrap())
            } else {
                format!("{}{next}", &url[..=url.rfind('/').unwrap()])
            };
        }
        panic!("No video fragment reached");
    }
    #[test]
    fn rejects_restricted_or_untrusted_hls() {
        assert_eq!(
            playlist_id("https://rutube.ru/plst/973044/?x=1"),
            Some("973044")
        );
        for url in [
            "https://rutube.ru.evil/plst/1/",
            "https://user@rutube.ru/plst/1/",
            "https://rutube.ru/plst/../",
            "http://rutube.ru/plst/1/",
        ] {
            assert_eq!(playlist_id(url), None);
        }
        assert!(valid_id("9906a8ef5c0a1f7dbcbc08e81ac63845"));
        assert!(!valid_id("../private"));
        for url in [
            "http://bl.rutube.ru/a",
            "https://rutube.ru.evil.test/a",
            "https://127.0.0.1/a",
            "https://user@bl.rutube.ru/a",
        ] {
            assert!(hls_from_options(&serde_json::json!({"video_balancer":{"m3u8":url}})).is_err());
        }
        let good = serde_json::json!({"acl_access":{"allowed":true},"video_balancer":{"m3u8":"https://bl.rutube.ru/a.m3u8?token=temporary"}});
        assert!(hls_from_options(&good).is_ok());
        let mut blocked = good;
        blocked["acl_access"]["allowed"] = false.into();
        assert!(hls_from_options(&blocked).is_err());
    }
}
