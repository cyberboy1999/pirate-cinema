use dioxus::prelude::*;
use pirate_cinema_core::covers::{self, CoverRequest};
use pirate_cinema_core::history::{
    create_local_backup, CollectionList, HistoryStore, MediaMetadata,
};
use pirate_cinema_core::metadata::Movie;
use pirate_cinema_core::remote_control;
use pirate_cinema_core::settings::{self, CoverMode, Language, Preferences};
use pirate_cinema_core::{
    check_rust_update, download_rust_update, ReleaseUpdate, SearchResult, Torrent, VideoFile,
};

use crate::{
    clock, default_preferences, file_episode, file_season, history_path, is_episode,
    launch_update_installer, png_data_uri, release_quality, save_detail_preferences, settings_path,
    short_hash, system_diagnostics, ContinueItem, LibraryCard, PlayableFile, ServerState, APP_ICON,
    INITIAL_PREFERENCES, VERSION,
};

#[component]
pub fn Icon(name: &'static str, size: u32) -> Element {
    let shape = match name {
        "home" => rsx! {
            path { d: "M3 10.8 12 3l9 7.8v8.4a1.8 1.8 0 0 1-1.8 1.8H4.8A1.8 1.8 0 0 1 3 19.2z" }
            path { d: "M9 21v-6h6v6" }
        },
        "search" => rsx! {
            circle { cx: "11", cy: "11", r: "7" }
            path { d: "m20 20-4.2-4.2" }
        },
        "library" => rsx! {
            rect { x: "3", y: "4", width: "7", height: "16", rx: "1.6" }
            rect { x: "14", y: "4", width: "7", height: "16", rx: "1.6" }
            path { d: "M6.5 8h0M6.5 12h0M6.5 16h0M17.5 8h0M17.5 12h0M17.5 16h0" }
        },
        "heart" => rsx! {
            path { d: "M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8l1.1 1.1L12 21l7.8-7.5 1.1-1.1a5.5 5.5 0 0 0-.1-7.8Z" }
        },
        "clock" => rsx! {
            circle { cx: "12", cy: "12", r: "9" }
            path { d: "M12 7v5l3 2" }
        },
        "history" => rsx! {
            path { d: "M3 12a9 9 0 1 0 2.6-6.3L3 8" }
            path { d: "M3 3v5h5M12 7v5l3 2" }
        },
        "settings" => rsx! {
            path { d: "M4 7h7M15 7h5M4 17h3M11 17h9" }
            circle { cx: "13", cy: "7", r: "2" }
            circle { cx: "9", cy: "17", r: "2" }
        },
        "play" => rsx! {
            path { fill: "currentColor", stroke: "none", d: "M8 5.5v13l10.5-6.5z" }
        },
        "plus" => rsx! {
            path { d: "M12 5v14M5 12h14" }
        },
        "check" => rsx! {
            path { d: "m5 12.5 4.2 4.2L19 7" }
        },
        "arrow-left" => rsx! {
            path { d: "M19 12H5M11 18l-6-6 6-6" }
        },
        "chevron-right" => rsx! {
            path { d: "m9 5 7 7-7 7" }
        },
        "x" => rsx! {
            path { d: "M6 6l12 12M18 6 6 18" }
        },
        "film" => rsx! {
            rect { x: "3", y: "4", width: "18", height: "16", rx: "2.4" }
            path { d: "M7 4v16M17 4v16M3 9h4M17 9h4M3 15h4M17 15h4" }
        },
        "tv" => rsx! {
            rect { x: "3", y: "7", width: "18", height: "13", rx: "2.4" }
            path { d: "m8 3 4 4 4-4" }
        },
        "refresh" => rsx! {
            path { d: "M20 11a8 8 0 1 0 2 5.3" }
            path { d: "M20 4v7h-7" }
        },
        "sliders" => rsx! {
            path { d: "M4 6h16M4 12h16M4 18h16" }
            circle { cx: "9", cy: "6", r: "1.5", fill: "currentColor" }
            circle { cx: "15", cy: "12", r: "1.5", fill: "currentColor" }
            circle { cx: "8", cy: "18", r: "1.5", fill: "currentColor" }
        },
        "info" => rsx! {
            circle { cx: "12", cy: "12", r: "9" }
            path { d: "M12 11v5M12 8h.01" }
        },
        "database" => rsx! {
            ellipse { cx: "12", cy: "5.5", rx: "7.5", ry: "3" }
            path { d: "M4.5 5.5v6c0 1.7 3.4 3 7.5 3s7.5-1.3 7.5-3v-6" }
            path { d: "M4.5 11.5v6c0 1.7 3.4 3 7.5 3s7.5-1.3 7.5-3v-6" }
        },
        "monitor" => rsx! {
            rect { x: "3", y: "4", width: "18", height: "13", rx: "2" }
            path { d: "M8 21h8M12 17v4" }
        },
        "wifi" => rsx! {
            path { d: "M5 12.5a10 10 0 0 1 14 0M8 15.5a6 6 0 0 1 8 0M11 18.5a2 2 0 0 1 2 0" }
            circle { cx: "12", cy: "21", r: ".7", fill: "currentColor" }
        },
        "download" => rsx! {
            path { d: "M12 3v12M7 10l5 5 5-5M5 21h14" }
        },
        "trash" => rsx! {
            path { d: "M4 7h16M10 11v6M14 11v6M6 7l1 14h10l1-14M9 7V4h6v3" }
        },
        "folder" => rsx! {
            path { d: "M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
        },
        "sparkles" => rsx! {
            path { d: "M12 3l1.3 4.2L17.5 8.5l-4.2 1.3L12 14l-1.3-4.2-4.2-1.3 4.2-1.3z" }
            path { d: "M18.5 14.5l.7 2.3 2.3.7-2.3.7-.7 2.3-.7-2.3-2.3-.7 2.3-.7z" }
        },
        "external" => rsx! {
            path { d: "M14 4h6v6M20 4l-9 9" }
            path { d: "M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" }
        },
        _ => rsx! { circle { cx: "12", cy: "12", r: "8" } },
    };
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            view_box: "0 0 24 24",
            width: "{size}",
            height: "{size}",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            {shape}
        }
    }
}

#[component]
pub fn NavButton(
    label: String,
    icon: &'static str,
    active: bool,
    count: Option<usize>,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: if active { "nav-item active" } else { "nav-item" },
            "aria-current": if active { "page" } else { "false" },
            title: "{label}",
            onclick: move |event| onclick.call(event),
            Icon { name: icon, size: 21 }
            span { "{label}" }
            if let Some(count) = count.filter(|count| *count > 0) {
                span { class: "nav-count", "{count}" }
            }
        }
    }
}

fn poster_art(poster: Option<&str>, title: &str, icon_size: u32) -> Element {
    let title = title.to_owned();
    match poster.filter(|poster| !poster.is_empty()) {
        Some(poster) => {
            let poster = poster.to_owned();
            rsx! { img { src: "{poster}", alt: "{title}", loading: "lazy", decoding: "async" } }
        }
        None => rsx! {
            div { class: "poster-fallback",
                Icon { name: "film", size: icon_size }
                span { "{title}" }
            }
        },
    }
}

/// Resolve the effective cover for a library card. The generated cover is
/// deterministic, so switching modes does not require rewriting SQLite or the
/// poster cache.
pub fn card_cover(card: &LibraryCard, mode: CoverMode) -> String {
    let metadata = card.metadata.as_ref();
    let genres = metadata
        .map(|item| item.genres.as_slice())
        .unwrap_or_default();
    let generated = covers::data_uri(&CoverRequest {
        hash: &card.torrent.hash,
        raw_title: &card.torrent.title,
        title: metadata.map(|item| item.title.as_str()),
        year: metadata.and_then(|item| item.year),
        rating: metadata.and_then(|item| item.rating),
        genres,
        media_type: card.media_type.as_deref(),
    });
    let poster = card.poster.as_deref().filter(|poster| !poster.is_empty());
    match mode {
        CoverMode::Generated => generated,
        CoverMode::Poster => poster.map(str::to_owned).unwrap_or(generated),
        CoverMode::Auto if metadata.is_some() => generated,
        CoverMode::Auto => poster.map(str::to_owned).unwrap_or(generated),
    }
}

/// Persist a cover choice and update the live preference signal so every
/// mounted card refreshes immediately.
fn select_cover_mode(
    mut cover_mode: Signal<CoverMode>,
    mut status: Signal<String>,
    language: Language,
    next: CoverMode,
) {
    cover_mode.set(next);
    status.set(
        match settings_path().and_then(|path| settings::save_cover_mode(&path, next)) {
            Ok(()) => language
                .pick("Обложки обновлены", "Cover style updated")
                .to_owned(),
            Err(error) => error,
        },
    );
}

fn media_title(card: &LibraryCard) -> String {
    card.metadata
        .as_ref()
        .map(|item| covers::display_title(&item.title))
        .unwrap_or_else(|| covers::display_title(&card.torrent.title))
}

fn media_subtitle(card: &LibraryCard, language: Language) -> String {
    if card.position > 0 && card.duration > 0 {
        return format!(
            "{} · {} / {}",
            language.pick("Продолжить", "Resume"),
            clock(card.position),
            clock(card.duration)
        );
    }
    let mut parts = Vec::new();
    if let Some(year) = card.metadata.as_ref().and_then(|item| item.year) {
        parts.push(year.to_string());
    }
    match card.media_type.as_deref() {
        Some("series") => parts.push(language.pick("Сериал", "Series").into()),
        Some("movie") => parts.push(language.pick("Фильм", "Film").into()),
        _ => {}
    }
    if card.viewed {
        parts.push(language.pick("Просмотрено", "Viewed").into());
    }
    if parts.is_empty() {
        format!("BTIH · {}", short_hash(&card.torrent.hash))
    } else {
        parts.join(" · ")
    }
}

fn progress_percent(position: i64, duration: i64) -> i64 {
    if position <= 0 || duration <= 0 {
        return 0;
    }
    (position.saturating_mul(100) / duration).clamp(0, 100)
}

fn file_label(file: &VideoFile) -> String {
    match (file_season(&file.path), file_episode(&file.path)) {
        (Some(season), Some(episode)) if season > 0 => format!("S{season:02}E{episode:02}"),
        (_, Some(episode)) => format!("E{episode:02}"),
        _ => format!("#{}", file.id + 1),
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return String::new();
    }
    let units = ["Б", "КБ", "МБ", "ГБ", "ТБ"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", value.round() as u64, units[unit])
    } else {
        format!("{value:.1} {}", units[unit])
    }
}

fn size_weight(value: &str) -> f64 {
    let normalized = value.replace(',', ".");
    let mut parts = normalized.split_whitespace();
    let amount = parts
        .next()
        .and_then(|part| part.parse::<f64>().ok())
        .unwrap_or(0.0);
    let unit = parts.next().unwrap_or_default().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "tb" | "тб" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "gb" | "гб" => 1024.0 * 1024.0 * 1024.0,
        "mb" | "мб" => 1024.0 * 1024.0,
        "kb" | "кб" => 1024.0,
        _ => 1.0,
    };
    amount * multiplier
}

#[component]
pub fn Welcome(on_complete: EventHandler<Preferences>) -> Element {
    let initial = INITIAL_PREFERENCES
        .get()
        .cloned()
        .unwrap_or_else(default_preferences);
    let mut english = use_signal(|| initial.language == Language::English);
    let brand_icon = png_data_uri(APP_ICON);
    let submit = move |_| {
        on_complete.call(Preferences {
            language: if english() {
                Language::English
            } else {
                Language::Russian
            },
            onboarding_complete: true,
            torznab_url: initial.torznab_url.clone(),
            torznab_api_key: initial.torznab_api_key.clone(),
            close_to_tray: initial.close_to_tray,
            cover_mode: initial.cover_mode,
        });
    };
    let title = if english() {
        "Your cinema stays with you."
    } else {
        "Кино, которое остаётся у вас."
    };
    let description = if english() {
        "A private library, a local torrent engine and a player in one calm place. Nothing about your collection or viewing history leaves this computer."
    } else {
        "Личная медиатека, локальный торрент-движок и плеер в одном спокойном пространстве. Медиатека и история просмотра не покидают этот компьютер."
    };
    rsx! {
        main { class: "welcome-shell",
            section { class: "welcome-copy",
                div { class: "welcome-brand",
                    img { src: "{brand_icon}", alt: "" }
                    span { "Pirate Cinema" }
                }
                h1 { "{title}" }
                p { "{description}" }
                ul { class: "welcome-points",
                    li { Icon { name: "database", size: 16 } span { if english() { "History and posters stay in local SQLite" } else { "История и постеры — в локальной SQLite" } } }
                    li { Icon { name: "film", size: 16 } span { if english() { "Embedded librqbit, no second server" } else { "Встроенный librqbit без второго сервера" } } }
                    li { Icon { name: "monitor", size: 16 } span { if english() { "ArtPlayer plays inside the app" } else { "ArtPlayer воспроизводит прямо в приложении" } } }
                }
            }
            section { class: "welcome-card",
                h2 { if english() { "Choose your language" } else { "Выберите язык" } }
                p { if english() { "You can change it later in Settings." } else { "Язык можно изменить позже в настройках." } }
                div { class: "language-choice", role: "group", "aria-label": if english() { "Language" } else { "Язык" },
                    button { class: if !english() { "active" } else { "" }, onclick: move |_| english.set(false), "Русский" }
                    button { class: if english() { "active" } else { "" }, onclick: move |_| english.set(true), "English" }
                }
                button { class: "button primary wide", onclick: submit,
                    if english() { "Enter Pirate Cinema" } else { "Войти в Pirate Cinema" }
                    Icon { name: "chevron-right", size: 17 }
                }
                p { class: "welcome-note",
                    if english() { "No account, no telemetry. Your library remains on this machine." } else { "Без аккаунта и телеметрии. Медиатека остаётся на этом компьютере." }
                }
            }
        }
    }
}

#[component]
pub fn MediaCard(
    card: LibraryCard,
    language: Language,
    on_open: EventHandler<Torrent>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
) -> Element {
    let title = media_title(&card);
    let subtitle = media_subtitle(&card, language);
    let progress = progress_percent(card.position, card.duration);
    let torrent = card.torrent.clone();
    let favorite = card.favorite;
    let watch_later = card.watch_later;
    let viewed = card.viewed;
    let media_type = card.media_type.clone();
    let cover_mode = use_context::<Signal<CoverMode>>();
    let cover = card_cover(&card, cover_mode());
    let open_torrent = torrent.clone();
    let favorite_torrent = torrent.clone();
    let later_torrent = torrent.clone();
    let favorite_label = if favorite {
        language.pick("Убрать из избранного", "Remove from favorites")
    } else {
        language.pick("В избранное", "Add to favorites")
    };
    let later_label = if watch_later {
        language.pick("Убрать из «Смотреть позже»", "Remove from watch later")
    } else {
        language.pick("Смотреть позже", "Watch later")
    };
    rsx! {
        article { class: "media-card",
            div { class: "media-poster",
                button {
                    class: "media-open",
                    title: if language == Language::Russian { "Открыть {title}" } else { "Open {title}" },
                    onclick: move |_| on_open.call(open_torrent.clone()),
                    {poster_art(Some(cover.as_str()), &title, 38)}
                }
                div { class: "media-actions",
                    button {
                        class: if favorite { "icon-button active" } else { "icon-button" },
                        title: "{favorite_label}",
                        "aria-label": "{favorite_label}",
                        onclick: move |_| on_collection.call((favorite_torrent.clone(), CollectionList::Favorites, !favorite)),
                        Icon { name: "heart", size: 16 }
                    }
                    button {
                        class: if watch_later { "icon-button active" } else { "icon-button" },
                        title: "{later_label}",
                        "aria-label": "{later_label}",
                        onclick: move |_| on_collection.call((later_torrent.clone(), CollectionList::WatchLater, !watch_later)),
                        Icon { name: "clock", size: 16 }
                    }
                }
                div { class: "media-badge",
                    if media_type.as_deref() == Some("series") {
                        span { Icon { name: "tv", size: 12 } if language == Language::Russian { "Сериал" } else { "Series" } }
                    } else if media_type.as_deref() == Some("movie") {
                        span { Icon { name: "film", size: 12 } if language == Language::Russian { "Фильм" } else { "Film" } }
                    }
                    if viewed {
                        span { class: "watched", Icon { name: "check", size: 12 } if language == Language::Russian { "Просмотрено" } else { "Viewed" } }
                    }
                }
                if progress > 0 {
                    div { class: "poster-progress", span { style: "width:{progress}%" } }
                }
            }
            button {
                class: "media-copy-button",
                onclick: move |_| on_open.call(torrent.clone()),
                div { class: "media-copy",
                    span { class: "media-title", "{title}" }
                    small { class: "media-subtitle", "{subtitle}" }
                }
            }
        }
    }
}

#[component]
pub fn MediaGrid(
    cards: Vec<LibraryCard>,
    language: Language,
    on_open: EventHandler<Torrent>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
) -> Element {
    rsx! {
        div { class: "media-grid",
            for card in cards {
                MediaCard {
                    key: "{card.torrent.hash}",
                    card: card.clone(),
                    language,
                    on_open,
                    on_collection,
                }
            }
        }
    }
}

#[component]
pub fn Home(
    language: Language,
    recent: Vec<ContinueItem>,
    cards: Vec<LibraryCard>,
    on_open: EventHandler<Torrent>,
    on_resume: EventHandler<ContinueItem>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
    on_browse: EventHandler<()>,
    on_search: EventHandler<()>,
    on_popular: EventHandler<String>,
) -> Element {
    let mut popular = use_resource(|| async {
        tokio::task::spawn_blocking(pirate_cinema_core::catalog::popular)
            .await
            .unwrap_or_else(|error| Err(error.to_string()))
    });
    let cover_mode = use_context::<Signal<CoverMode>>();
    let available = cards.clone();
    let recently_played = recent
        .iter()
        .filter_map(|item| {
            available
                .iter()
                .find(|card| card.torrent.hash.eq_ignore_ascii_case(&item.torrent.hash))
                .cloned()
                .map(|card| (item.clone(), card))
        })
        .collect::<Vec<_>>();
    let featured = recently_played.first().cloned().or_else(|| {
        available.first().cloned().map(|card| {
            let item = ContinueItem {
                torrent: card.torrent.clone(),
                title: media_title(&card),
                file_name: String::new(),
                file_index: 0,
                position: card.position,
                duration: card.duration,
            };
            (item, card)
        })
    });
    let favorites = available
        .iter()
        .filter(|card| card.favorite)
        .cloned()
        .collect::<Vec<_>>();
    let continue_items = recently_played.iter().skip(1).cloned().collect::<Vec<_>>();

    rsx! {
        section { class: "screen home-screen",
            if let Some((item, card)) = featured {
                {
                    let title = media_title(&card);
                    let overview = card.metadata.as_ref().and_then(|metadata| metadata.overview.clone());
                    let year = card.metadata.as_ref().and_then(|metadata| metadata.year);
                    let rating = card.metadata.as_ref().and_then(|metadata| metadata.rating);
                    let genres = card.metadata.as_ref().map(|metadata| metadata.genres.clone()).unwrap_or_default();
                    let poster = Some(card_cover(&card, cover_mode()));
                    let progress = progress_percent(card.position, card.duration);
                    let favorite = card.favorite;
                    let open_torrent = card.torrent.clone();
                    let collection_torrent = card.torrent.clone();
                    let resume_item = item.clone();
                    let has_resume = card.position > 0 && card.duration > 0;
                    let primary_label = if has_resume {
                        language.pick("Продолжить", "Resume")
                    } else {
                        language.pick("Смотреть", "Watch")
                    };
                    let kicker = if has_resume {
                        language.pick("Продолжить просмотр", "Continue watching")
                    } else {
                        language.pick("Из вашей медиатеки", "From your library")
                    };
                    let kicker_icon = if has_resume { "clock" } else { "sparkles" };
                    let favorite_label = if favorite {
                        language.pick("Убрать из избранного", "Remove from favorites")
                    } else {
                        language.pick("В избранное", "Add to favorites")
                    };
                    rsx! {
                        section { class: "spotlight", onmousemove: move |event| {
                            let _ = event;
                        },
                            if let Some(poster) = poster.as_ref() {
                                div { class: "spotlight-art", style: "background-image:url('{poster}');" }
                            }
                            div { class: "spotlight-copy",
                                div { class: "spotlight-kicker",
                                    span { class: "line" }
                                    Icon { name: kicker_icon, size: 15 }
                                    span { "{kicker}" }
                                }
                                h1 { "{title}" }
                                div { class: "meta-row",
                                    if let Some(year) = year { span { class: "meta-pill", "{year}" } }
                                    if let Some(rating) = rating { span { class: "meta-pill accent", "★ {rating:.1}" } }
                                    if card.media_type.as_deref() == Some("series") { span { class: "meta-pill", {language.pick("Сериал", "Series")} } }
                                    for genre in genres.iter().take(2) { span { class: "meta-pill", "{genre}" } }
                                }
                                if let Some(overview) = overview {
                                    p { class: "spotlight-overview", "{overview}" }
                                } else {
                                    p { class: "spotlight-overview", {language.pick("Описание ещё не загружено. Откройте карточку, чтобы увидеть файлы и запустить просмотр.", "The description has not been loaded yet. Open the title to see its files and start watching.")} }
                                }
                                if has_resume {
                                    div { class: "spotlight-progress",
                                        span { "{clock(item.position)}" }
                                        div { class: "progress-track", span { style: "width:{progress}%" } }
                                        span { "{clock(item.duration)}" }
                                    }
                                }
                                div { class: "spotlight-actions",
                                    button {
                                        class: "button primary",
                                        onclick: {
                                            let open_torrent = open_torrent.clone();
                                            let resume_item = resume_item.clone();
                                            move |_| {
                                                if has_resume {
                                                    on_resume.call(resume_item.clone());
                                                } else {
                                                    on_open.call(open_torrent.clone());
                                                }
                                            }
                                        },
                                        Icon { name: "play", size: 17 }
                                        "{primary_label}"
                                    }
                                    button {
                                        class: "button secondary",
                                        onclick: {
                                            let open_torrent = open_torrent.clone();
                                            move |_| on_open.call(open_torrent.clone())
                                        },
                                        Icon { name: "info", size: 17 }
                                        {language.pick("Подробнее", "Details")}
                                    }
                                    button {
                                        class: if favorite { "icon-button active" } else { "icon-button" },
                                        title: "{favorite_label}",
                                        onclick: move |_| on_collection.call((collection_torrent.clone(), CollectionList::Favorites, !favorite)),
                                        Icon { name: "heart", size: 17 }
                                    }
                                }
                            }
                            div { class: "spotlight-poster", {poster_art(poster.as_deref(), &title, 52)} }
                        }
                    }
                }
            } else {
                div { class: "empty-state",
                    div { class: "empty-state-icon", Icon { name: "film", size: 30 } }
                    h2 { {language.pick("Здесь начнётся ваша медиатека", "Your library starts here")} }
                    p { {language.pick("Найдите фильм или сериал, добавьте magnet-ссылку или откройте поиск. Всё останется на этом компьютере.", "Find a film or series, add a magnet link, or open search. Everything stays on this computer.")} }
                    div { class: "heading-actions",
                        button { class: "button primary", onclick: move |_| on_search.call(()), Icon { name: "search", size: 17 } {language.pick("Найти фильм", "Find a film")} }
                        button { class: "button secondary", onclick: move |_| on_browse.call(()), Icon { name: "library", size: 17 } {language.pick("Медиатека", "Library")} }
                    }
                }
            }

            if !continue_items.is_empty() {
                section { class: "shelf",
                    div { class: "shelf-head",
                        h2 { {language.pick("Продолжить просмотр", "Continue watching")} }
                        span { class: "meta-pill", "{continue_items.len()}" }
                    }
                    div { class: "shelf-rail",
                        for (item, card) in continue_items {
                            {
                                let title = item.title.clone();
                                let file_name = item.file_name.clone();
                                let poster = card_cover(&card, cover_mode());
                                let progress = progress_percent(item.position, item.duration);
                                let resume = item.clone();
                                let card_style = format!("background-image:url('{poster}')");
                                rsx! {
                                    button {
                                        key: "{item.torrent.hash}",
                                        class: "continue-card",
                                        style: "{card_style}",
                                        onclick: move |_| on_resume.call(resume.clone()),
                                        span { class: "play-orb", Icon { name: "play", size: 18 } }
                                        strong { "{title}" }
                                        small { "{file_name}" }
                                        div { class: "progress-track", span { style: "width:{progress}%" } }
                                        div { class: "continue-time",
                                            span { "{clock(item.position)}" }
                                            span { "{clock(item.duration)}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !available.is_empty() {
                section { class: "shelf",
                    div { class: "shelf-head",
                        h2 { {language.pick("Новое в медиатеке", "New in the library")} }
                        button { class: "button ghost", onclick: move |_| on_browse.call(()),
                            {language.pick("Вся медиатека", "All titles")}
                            Icon { name: "chevron-right", size: 15 }
                        }
                    }
                    div { class: "shelf-rail posters",
                        for card in available.iter().take(12).cloned() {
                            MediaCard {
                                key: "new-{card.torrent.hash}",
                                card: card.clone(),
                                language,
                                on_open,
                                on_collection,
                            }
                        }
                    }
                }
            }

            section { class: "shelf",
                div { class: "shelf-head",
                    h2 { {language.pick("Популярные фильмы", "Popular movies")} }
                    span { class: "meta-pill", "Cinemeta" }
                }
                match popular.read().as_ref() {
                    None => rsx! { p { class: "hint", {language.pick("Загрузка фильмов…", "Loading movies…")} } },
                    Some(Err(_)) => rsx! {
                        button { class: "button secondary", onclick: move |_| popular.restart(),
                            {language.pick("Повторить загрузку", "Retry loading")}
                        }
                    },
                    Some(Ok(movies)) => rsx! {
                        div { class: "shelf-rail posters",
                            for movie in movies.iter().take(18) {
                                article { class: "media-card", key: "popular-{movie.id}",
                                    button { class: "popular-poster", title: "{movie.title}",
                                        onclick: { let title = movie.title.clone(); move |_| on_popular.call(title.clone()) },
                                        if let Some(poster) = &movie.poster_url {
                                            img { src: "{poster}", alt: "{movie.title}", loading: "lazy" }
                                        }
                                        strong { "{movie.title}" }
                                        if let Some(year) = movie.year { small { "{year}" } }
                                    }
                                }
                            }
                        }
                    },
                }
            }
            if !favorites.is_empty() {
                section { class: "shelf",
                    div { class: "shelf-head",
                        h2 { {language.pick("Избранное", "Favorites")} }
                        span { class: "meta-pill", "{favorites.len()}" }
                    }
                    div { class: "shelf-rail posters",
                        for card in favorites.iter().take(12).cloned() {
                            MediaCard {
                                key: "favorite-{card.torrent.hash}",
                                card: card.clone(),
                                language,
                                on_open,
                                on_collection,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SearchPage(
    language: Language,
    query: String,
    metadata: Option<Movie>,
    poster: String,
    results: Vec<SearchResult>,
    busy: bool,
    on_add: EventHandler<(SearchResult, bool)>,
) -> Element {
    let cover_mode = use_context::<Signal<CoverMode>>();
    let mut quality = use_signal(|| "all".to_owned());
    let mut source_filter = use_signal(|| "all".to_owned());
    let mut order = use_signal(|| "seeders".to_owned());
    let mut qualities = results
        .iter()
        .map(|item| release_quality(&item.title))
        .collect::<Vec<_>>();
    qualities.sort_unstable();
    qualities.dedup();
    let mut sources = results
        .iter()
        .map(|item| item.source.clone())
        .filter(|source| !source.is_empty())
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    let selected_quality = if qualities.contains(&quality().as_str()) {
        quality()
    } else {
        "all".to_owned()
    };
    let selected_source = if sources.contains(&source_filter()) {
        source_filter()
    } else {
        "all".to_owned()
    };
    let selected_order = order();
    let mut visible = results
        .iter()
        .filter(|item| {
            (selected_quality == "all" || release_quality(&item.title) == selected_quality)
                && (selected_source == "all" || item.source == selected_source)
        })
        .cloned()
        .collect::<Vec<_>>();
    match selected_order.as_str() {
        "size" => visible.sort_by(|left, right| {
            size_weight(&right.size)
                .partial_cmp(&size_weight(&left.size))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "title" => visible.sort_by_key(|item| item.title.to_lowercase()),
        _ => visible.sort_by_key(|item| std::cmp::Reverse(item.seeders)),
    }
    let title = metadata
        .as_ref()
        .map_or_else(|| query.clone(), |item| covers::display_title(&item.title));
    let generated = if query.is_empty() && metadata.is_none() {
        String::new()
    } else {
        let item = metadata.as_ref();
        covers::data_uri(&CoverRequest {
            hash: item.map_or(query.as_str(), |movie| movie.id.as_str()),
            raw_title: &query,
            title: item.map(|movie| movie.title.as_str()),
            year: item.and_then(|movie| movie.year),
            rating: item.and_then(|movie| movie.rating),
            genres: item
                .map(|movie| movie.genres.as_slice())
                .unwrap_or_default(),
            media_type: None,
        })
    };
    let cover = match cover_mode() {
        CoverMode::Generated => generated.clone(),
        CoverMode::Poster if !poster.is_empty() => poster.clone(),
        CoverMode::Auto if metadata.is_none() && !poster.is_empty() => poster.clone(),
        _ => generated,
    };
    rsx! {
        section { class: "screen search-screen",
            div { class: "search-main",
                div { class: "screen-heading",
                    div {
                        h1 { {language.pick("Поиск", "Search")} }
                        p {
                            if query.is_empty() {
                                {language.pick("Введите название фильма, сериала или magnet-ссылку в строке сверху.", "Enter a film, series, or magnet link in the search field above.")}
                            } else if busy {
                                {language.pick("Ищем раздачи и подбираем описание…", "Searching releases and matching metadata…")}
                            } else {
                                if language == Language::Russian { "Найдено раздач: {results.len()}. Выберите подходящую." } else { "Releases found: {results.len()}. Choose one." }
                            }
                        }
                    }
                }
                if !results.is_empty() {
                    div { class: "search-filters",
                        div { class: "chip-row",
                            button { class: if selected_quality == "all" { "chip active" } else { "chip" }, onclick: move |_| quality.set("all".into()), {language.pick("Все", "All")} }
                            for item in qualities {
                                button {
                                    class: if selected_quality == item { "chip active" } else { "chip" },
                                    onclick: { let item = item.to_owned(); move |_| quality.set(item.clone()) },
                                    if item == "Другое" { {language.pick("Другое", "Other")} } else { "{item}" }
                                }
                            }
                        }
                        if sources.len() > 1 {
                            select { class: "select", value: "{selected_source}", onchange: move |event| source_filter.set(event.value()),
                                option { value: "all", {language.pick("Все источники", "All sources")} }
                                for source in sources { option { value: "{source}", "{source}" } }
                            }
                        }
                        select { class: "select search-sort", value: "{selected_order}", onchange: move |event| order.set(event.value()),
                            option { value: "seeders", {language.pick("Сначала сиды", "Most seeders")} }
                            option { value: "size", {language.pick("Сначала размер", "Largest first")} }
                            option { value: "title", {language.pick("По названию", "By title")} }
                        }
                    }
                }
                if busy && results.is_empty() {
                    div { class: "skeleton-list",
                        div { class: "skeleton-row" }
                        div { class: "skeleton-row" }
                        div { class: "skeleton-row" }
                        div { class: "skeleton-row" }
                    }
                } else if visible.is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", Icon { name: "search", size: 30 } }
                        h2 { if query.is_empty() { {language.pick("Найдите что-нибудь для вечера", "Find something for tonight")} } else { {language.pick("Подходящих раздач нет", "No matching releases")} } }
                        p { if query.is_empty() { {language.pick("Поиск идёт по настроенным Torznab/Jackett источникам. Можно также вставить magnet-ссылку.", "Search runs across the configured Torznab/Jackett sources. You can also paste a magnet link.")} } else { {language.pick("Попробуйте изменить запрос или сбросить фильтр качества.", "Try another query or clear the quality filter.")} } }
                    }
                } else {
                    div { class: "result-list",
                        for item in visible {
                            article { class: "result-row",
                                div { class: "result-main",
                                    strong { title: "{item.title}", "{item.title}" }
                                    div { class: "result-meta",
                                        span { class: "meta-pill", "[{release_quality(&item.title)}]" }
                                        if !item.source.is_empty() { span { class: "source", "{item.source}" } }
                                        if !item.tracker.is_empty() { span { "{item.tracker}" } }
                                        span { "{item.size}" }
                                        if language == Language::Russian { span { "{item.seeders} сидов" } } else { span { "{item.seeders} seeders" } }
                                    }
                                }
                                span { class: "result-size", "{item.size}" }
                                span { class: "result-seeders", if language == Language::Russian { "{item.seeders} сидов" } else { "{item.seeders} seeders" } }
                                div { class: "result-actions",
                                    button {
                                        class: "button secondary small",
                                        disabled: busy,
                                        onclick: { let item = item.clone(); move |_| on_add.call((item.clone(), false)) },
                                        {language.pick("Добавить", "Add")}
                                    }
                                    button {
                                        class: "button primary small",
                                        disabled: busy,
                                        onclick: { let item = item.clone(); move |_| on_add.call((item.clone(), true)) },
                                        Icon { name: "play", size: 14 }
                                        {language.pick("Смотреть", "Watch")}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            aside { class: "search-preview",
                if !cover.is_empty() {
                    div { class: "search-preview-poster", img { src: "{cover}", alt: "{title}" } }
                } else {
                    div { class: "search-preview-empty",
                        Icon { name: "film", size: 32 }
                        span { if query.is_empty() { {language.pick("Карточка фильма появится здесь", "The title card appears here")} } else { {language.pick("Описание пока не найдено", "No description found yet")} } }
                    }
                }
                if metadata.is_some() || !query.is_empty() {
                    div { class: "search-preview-copy",
                        h2 { "{title}" }
                        div { class: "meta-row",
                            if let Some(item) = metadata.as_ref() {
                                if let Some(year) = item.year { span { class: "meta-pill", "{year}" } }
                                if let Some(rating) = item.rating { span { class: "meta-pill accent", "★ {rating:.1}" } }
                            }
                        }
                        if let Some(overview) = metadata.as_ref().and_then(|item| item.overview.as_ref()) {
                            p { "{overview}" }
                        } else if !query.is_empty() {
                            p { {language.pick("Выберите раздачу справа — описание и постер подтянутся из локального кэша.", "Choose a release to the left; the description and poster come from the local cache.")} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Library(
    language: Language,
    cards: Vec<LibraryCard>,
    busy: bool,
    status: String,
    on_open: EventHandler<Torrent>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
    on_sync: EventHandler<MouseEvent>,
) -> Element {
    let mut filter = use_signal(String::new);
    let mut kind = use_signal(|| "all".to_owned());
    let mut watched = use_signal(|| "all".to_owned());
    let mut genre = use_signal(|| "all".to_owned());
    let mut year_from = use_signal(String::new);
    let mut year_to = use_signal(String::new);
    let mut order = use_signal(|| "recent".to_owned());
    let mut favorite_only = use_signal(|| false);
    let mut watch_later_only = use_signal(|| false);
    let mut show_filters = use_signal(|| false);
    let mut genres = cards
        .iter()
        .flat_map(|card| {
            card.metadata
                .as_ref()
                .into_iter()
                .flat_map(|item| item.genres.iter().cloned())
        })
        .collect::<Vec<_>>();
    genres.sort();
    genres.dedup();
    let mut years = cards
        .iter()
        .filter_map(|card| card.metadata.as_ref().and_then(|item| item.year))
        .collect::<Vec<_>>();
    years.sort_unstable_by(|left, right| right.cmp(left));
    years.dedup();
    let needle = filter().trim().to_lowercase();
    let kind_value = kind();
    let watched_value = watched();
    let genre_value = genre();
    let from_value = year_from().trim().parse::<i64>().ok();
    let to_value = year_to().trim().parse::<i64>().ok();
    let favorite_value = favorite_only();
    let later_value = watch_later_only();
    let mut visible = cards
        .iter()
        .filter(|card| {
            let title = media_title(card);
            let year = card.metadata.as_ref().and_then(|item| item.year);
            (needle.is_empty() || title.to_lowercase().contains(&needle))
                && (kind_value == "all" || card.media_type.as_deref() == Some(kind_value.as_str()))
                && (genre_value == "all"
                    || card
                        .metadata
                        .as_ref()
                        .is_some_and(|item| item.genres.iter().any(|item| item == &genre_value)))
                && from_value.is_none_or(|from| year.is_some_and(|year| year >= from))
                && to_value.is_none_or(|to| year.is_some_and(|year| year <= to))
                && (!favorite_value || card.favorite)
                && (!later_value || card.watch_later)
                && match watched_value.as_str() {
                    "viewed" => card.viewed,
                    "unviewed" => !card.viewed,
                    _ => true,
                }
        })
        .cloned()
        .collect::<Vec<_>>();
    match order().as_str() {
        "title" => visible.sort_by_key(|card| media_title(card).to_lowercase()),
        "year" => visible.sort_by_key(|card| {
            std::cmp::Reverse(
                card.metadata
                    .as_ref()
                    .and_then(|item| item.year)
                    .unwrap_or(0),
            )
        }),
        "rating" => visible.sort_by(|left, right| {
            let left = left
                .metadata
                .as_ref()
                .and_then(|item| item.rating)
                .unwrap_or(0.0);
            let right = right
                .metadata
                .as_ref()
                .and_then(|item| item.rating)
                .unwrap_or(0.0);
            right
                .partial_cmp(&left)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => {}
    }
    let active_filters = usize::from(!needle.is_empty())
        + usize::from(kind_value != "all")
        + usize::from(watched_value != "all")
        + usize::from(genre_value != "all")
        + usize::from(from_value.is_some())
        + usize::from(to_value.is_some())
        + usize::from(favorite_value)
        + usize::from(later_value);
    let reset_filters = move |_| {
        filter.set(String::new());
        kind.set("all".into());
        watched.set("all".into());
        genre.set("all".into());
        year_from.set(String::new());
        year_to.set(String::new());
        favorite_only.set(false);
        watch_later_only.set(false);
    };
    rsx! {
        section { class: "screen library-screen",
            div { class: "screen-heading",
                div {
                    h1 { {language.pick("Медиатека", "Library")} }
                    p {
                        if language == Language::Russian { "Раздач: {cards.len()}. Метаданные, постеры и история хранятся в локальной SQLite." } else { "Titles: {cards.len()}. Metadata, posters, and history stay in local SQLite." }
                    }
                }
                div { class: "heading-actions",
                    button { class: "button secondary", disabled: busy, onclick: move |event| on_sync.call(event),
                        Icon { name: "refresh", size: 16 }
                        if busy { {language.pick("Обновляем…", "Updating…")} } else { {language.pick("Обновить карточки", "Refresh metadata")} }
                    }
                }
            }
            if !status.is_empty() { div { class: "app-notice", "{status}" } }
            div { class: "library-toolbar",
                div { class: "library-toolbar-main",
                    label { class: "input-wrap",
                        Icon { name: "search", size: 17 }
                        input {
                            class: "text-input",
                            value: "{filter}",
                            placeholder: language.pick("Фильтр по названию", "Filter by title"),
                            oninput: move |event| filter.set(event.value())
                        }
                    }
                    select { class: "select", value: "{kind_value}", onchange: move |event| kind.set(event.value()),
                        option { value: "all", {language.pick("Все типы", "All types")} }
                        option { value: "movie", {language.pick("Фильмы", "Movies")} }
                        option { value: "series", {language.pick("Сериалы", "Series")} }
                    }
                    select { class: "select", value: "{watched_value}", onchange: move |event| watched.set(event.value()),
                        option { value: "all", {language.pick("Любой статус", "Any status")} }
                        option { value: "viewed", {language.pick("Просмотрено", "Viewed")} }
                        option { value: "unviewed", {language.pick("Не просмотрено", "Not viewed")} }
                    }
                    div { class: "chip-row",
                        button {
                            class: if show_filters() { "chip active" } else { "chip" },
                            onclick: move |_| show_filters.set(!show_filters()),
                            Icon { name: "sliders", size: 15 }
                            {language.pick("Фильтры", "Filters")}
                            if active_filters > 0 { span { "{active_filters}" } }
                        }
                    }
                }
                if show_filters() {
                    div { class: "filter-panel",
                        label { class: "field-label", {language.pick("Жанр", "Genre")}
                            select { class: "select", value: "{genre_value}", onchange: move |event| genre.set(event.value()),
                                option { value: "all", {language.pick("Любой жанр", "Any genre")} }
                                for item in genres { option { value: "{item}", "{item}" } }
                            }
                        }
                        label { class: "field-label", {language.pick("Год от", "Year from")}
                            input { class: "text-input", r#type: "number", value: "{year_from}", placeholder: "1990", oninput: move |event| year_from.set(event.value()) }
                        }
                        label { class: "field-label", {language.pick("Год до", "Year to")}
                            input { class: "text-input", r#type: "number", value: "{year_to}", placeholder: "2026", oninput: move |event| year_to.set(event.value()) }
                        }
                        label { class: "field-label", {language.pick("Сортировка", "Sort by")}
                            select { class: "select", value: "{order}", onchange: move |event| order.set(event.value()),
                                option { value: "recent", {language.pick("Недавно добавленные", "Recently added")} }
                                option { value: "title", {language.pick("По названию", "By title")} }
                                option { value: "year", {language.pick("По году", "By year")} }
                                option { value: "rating", {language.pick("По рейтингу", "By rating")} }
                            }
                        }
                        div { class: "chip-row",
                            button { class: if favorite_value { "chip active" } else { "chip" }, onclick: move |_| favorite_only.set(!favorite_value),
                                Icon { name: "heart", size: 14 }
                                {language.pick("Избранное", "Favorites")}
                            }
                            button { class: if later_value { "chip active" } else { "chip" }, onclick: move |_| watch_later_only.set(!later_value),
                                Icon { name: "clock", size: 14 }
                                {language.pick("Позже", "Watch later")}
                            }
                        }
                    }
                    div { class: "filter-footer",
                        span { class: "filter-count", if language == Language::Russian { "Показано: {visible.len()} из {cards.len()}" } else { "Showing {visible.len()} of {cards.len()}" } }
                        if active_filters > 0 {
                            button { class: "button ghost small", onclick: reset_filters, Icon { name: "x", size: 14 } {language.pick("Сбросить", "Reset")} }
                        }
                    }
                }
            }
            if visible.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-state-icon", Icon { name: "library", size: 30 } }
                    h2 { if cards.is_empty() { {language.pick("Медиатека пока пуста", "The library is empty")} } else { {language.pick("Ничего не найдено", "Nothing matches")} } }
                    p { if cards.is_empty() { {language.pick("Добавьте фильм или сериал через поиск. Постеры и описания появятся здесь.", "Add a film or series through search. Posters and descriptions appear here.")} } else { {language.pick("Измените запрос или сбросьте фильтры.", "Change the query or reset the filters.")} } }
                    if active_filters > 0 {
                        button { class: "button secondary", onclick: reset_filters, {language.pick("Сбросить фильтры", "Reset filters")} }
                    }
                }
            } else {
                MediaGrid { cards: visible, language, on_open, on_collection }
            }
        }
    }
}

#[component]
pub fn CollectionPage(
    language: Language,
    cards: Vec<LibraryCard>,
    list: CollectionList,
    on_open: EventHandler<Torrent>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
    on_browse: EventHandler<()>,
) -> Element {
    let is_favorites = list == CollectionList::Favorites;
    let title = if is_favorites {
        language.pick("Избранное", "Favorites")
    } else {
        language.pick("Смотреть позже", "Watch later")
    };
    let description = if is_favorites {
        language.pick(
            "Фильмы и сериалы, к которым хочется вернуться.",
            "Films and series worth returning to.",
        )
    } else {
        language.pick(
            "Ваша личная очередь на следующий свободный вечер.",
            "A personal queue for the next free evening.",
        )
    };
    let visible = cards
        .into_iter()
        .filter(|card| {
            if is_favorites {
                card.favorite
            } else {
                card.watch_later
            }
        })
        .collect::<Vec<_>>();
    let count_label = if language == Language::Russian {
        format!("{} тайтлов", visible.len())
    } else {
        format!("{} titles", visible.len())
    };
    let collection_icon = if is_favorites { "heart" } else { "clock" };
    let empty_title = if is_favorites {
        language.pick("Избранное пока пусто", "No favorites yet")
    } else {
        language.pick("Очередь пока пуста", "Your queue is empty")
    };
    let empty_body = if is_favorites {
        language.pick(
            "Нажмите на сердце у любого тайтла, чтобы сохранить его здесь.",
            "Tap the heart on any title to keep it here.",
        )
    } else {
        language.pick(
            "Отметьте фильм или сериал часовой кнопкой, чтобы вернуться к нему позже.",
            "Use the clock button on any title to come back to it later.",
        )
    };
    rsx! {
        section { class: "screen collection-screen",
            div { class: "collection-hero",
                div {
                    h1 { "{title}" }
                    p { "{description}" }
                    if !visible.is_empty() { div { class: "meta-row", span { class: "meta-pill", "{count_label}" } } }
                }
                div { class: "collection-icon", Icon { name: collection_icon, size: 30 } }
            }
            if visible.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-state-icon", Icon { name: collection_icon, size: 30 } }
                    h2 { "{empty_title}" }
                    p { "{empty_body}" }
                    button { class: "button primary", onclick: move |_| on_browse.call(()), Icon { name: "library", size: 17 } {language.pick("Открыть медиатеку", "Open library")} }
                }
            } else {
                MediaGrid { cards: visible, language, on_open, on_collection }
            }
        }
    }
}

#[component]
pub fn HistoryPage(
    language: Language,
    recent: Vec<ContinueItem>,
    cards: Vec<LibraryCard>,
    on_open: EventHandler<Torrent>,
    on_resume: EventHandler<ContinueItem>,
    on_browse: EventHandler<()>,
) -> Element {
    let cover_mode = use_context::<Signal<CoverMode>>();
    rsx! {
        section { class: "screen history-screen",
            div { class: "screen-heading",
                div {
                    h1 { {language.pick("История", "History")} }
                    p { {language.pick("Продолжите с последнего места. История хранится только в локальной базе.", "Resume where you left off. History is stored only in the local database.")} }
                }
                div { class: "heading-actions",
                    button { class: "button secondary", onclick: move |_| on_browse.call(()), Icon { name: "library", size: 16 } {language.pick("Медиатека", "Library")} }
                }
            }
            if recent.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-state-icon", Icon { name: "history", size: 30 } }
                    h2 { {language.pick("История пока пуста", "No history yet")} }
                    p { {language.pick("Запустите первый фильм или серию — здесь появится позиция и прогресс.", "Play your first film or episode, and its position and progress will appear here.")} }
                    button { class: "button primary", onclick: move |_| on_browse.call(()), {language.pick("Открыть медиатеку", "Open library")} }
                }
            } else {
                div { class: "history-list",
                    for item in recent {
                        {
                            let card = cards.iter().find(|card| card.torrent.hash.eq_ignore_ascii_case(&item.torrent.hash)).cloned();
                            let poster = card.as_ref().map(|card| card_cover(card, cover_mode()));
                            let title = card.as_ref().map_or_else(
                                || covers::display_title(&item.title),
                                media_title,
                            );
                            let media_type = card.as_ref().and_then(|card| card.media_type.clone());
                            let progress = progress_percent(item.position, item.duration);
                            let resume = item.clone();
                            let open = item.torrent.clone();
                            rsx! {
                                article { class: "history-row", key: "{item.torrent.hash}",
                                    div { class: "history-poster", {poster_art(poster.as_deref(), &title, 28)} }
                                    div { class: "history-copy",
                                        strong { "{title}" }
                                        small { "{item.file_name}" }
                                        div { class: "history-progress",
                                            span { "{clock(item.position)}" }
                                            div { class: "progress-track", span { style: "width:{progress}%" } }
                                            span { "{clock(item.duration)}" }
                                            if media_type.as_deref() == Some("series") { span { {language.pick("Сериал", "Series")} } }
                                        }
                                    }
                                    div { class: "history-actions",
                                        button { class: "button secondary", onclick: move |_| on_open.call(open.clone()), {language.pick("Карточка", "Details")} }
                                        button { class: "button primary", onclick: move |_| on_resume.call(resume.clone()), Icon { name: "play", size: 14 } {language.pick("Продолжить", "Resume")} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Detail(
    language: Language,
    torrent: Option<Torrent>,
    metadata: Option<MediaMetadata>,
    media_type: Option<String>,
    poster: Option<String>,
    favorite: bool,
    watch_later: bool,
    files: Vec<PlayableFile>,
    busy: bool,
    metadata_status: String,
    on_back: EventHandler<MouseEvent>,
    on_play: EventHandler<(VideoFile, bool, bool)>,
    on_media_type: EventHandler<String>,
    on_title: EventHandler<String>,
    on_refresh_metadata: EventHandler<MouseEvent>,
    on_remove: EventHandler<MouseEvent>,
    on_collection: EventHandler<(Torrent, CollectionList, bool)>,
) -> Element {
    let saved_preferences = torrent
        .as_ref()
        .and_then(|torrent| {
            history_path().ok().and_then(|path| {
                HistoryStore::open(&path)
                    .ok()?
                    .playback_preferences(&torrent.hash)
                    .ok()
            })
        })
        .unwrap_or_default();
    let mut confirm_remove = use_signal(|| false);
    let mut file_filter = use_signal(String::new);
    let mut season = use_signal(|| {
        saved_preferences
            .season
            .map_or_else(|| "all".into(), |value| value.to_string())
    });
    let mut episode = use_signal(|| {
        saved_preferences
            .episode
            .map_or_else(|| "all".into(), |value| value.to_string())
    });
    let mut file_kind = use_signal(|| "all".to_owned());
    let mut auto_next = use_signal(|| saved_preferences.auto_next);
    let Some(torrent) = torrent else {
        return rsx! { section { class: "screen", div { class: "empty-state", h2 { {language.pick("Раздача не выбрана", "No torrent selected")} } } } };
    };
    let title = metadata.as_ref().map_or_else(
        || covers::display_title(&torrent.title),
        |item| covers::display_title(&item.title),
    );
    let mut title_edit = use_signal(|| title.clone());
    let needle = file_filter().trim().to_lowercase();
    let mut seasons = files
        .iter()
        .filter_map(|item| file_season(&item.file.path))
        .collect::<Vec<_>>();
    seasons.sort_unstable();
    seasons.dedup();
    let season_progress = seasons
        .iter()
        .map(|number| {
            let episodes = files
                .iter()
                .filter(|item| file_season(&item.file.path) == Some(*number))
                .collect::<Vec<_>>();
            (
                *number,
                episodes.iter().filter(|item| item.viewed).count(),
                episodes.len(),
            )
        })
        .collect::<Vec<_>>();
    let mut episodes = files
        .iter()
        .filter(|item| {
            season() == "all"
                || file_season(&item.file.path)
                    .map(|value| value.to_string())
                    .as_deref()
                    == Some(season().as_str())
        })
        .filter_map(|item| file_episode(&item.file.path))
        .collect::<Vec<_>>();
    episodes.sort_unstable();
    episodes.dedup();
    let has_movies = files.iter().any(|item| !is_episode(&item.file.path));
    let has_episodes = files.iter().any(|item| is_episode(&item.file.path));
    let files_empty = files.is_empty();
    let has_next_file = files.len() > 1;
    let mut visible_files = files
        .iter()
        .filter(|item| {
            (needle.is_empty()
                || item.file.name.to_lowercase().contains(&needle)
                || item.file.path.to_lowercase().contains(&needle))
                && (season() == "all"
                    || file_season(&item.file.path)
                        .map(|value| value.to_string())
                        .as_deref()
                        == Some(season().as_str()))
                && (episode() == "all"
                    || file_episode(&item.file.path)
                        .map(|value| value.to_string())
                        .as_deref()
                        == Some(episode().as_str()))
                && match file_kind().as_str() {
                    "movies" => !is_episode(&item.file.path),
                    "episodes" => is_episode(&item.file.path),
                    _ => true,
                }
        })
        .cloned()
        .collect::<Vec<_>>();
    visible_files.sort_by_key(|item| {
        (
            is_episode(&item.file.path),
            file_season(&item.file.path).unwrap_or(0),
            file_episode(&item.file.path).unwrap_or(0),
            item.file.path.to_lowercase(),
        )
    });
    let next_file = visible_files
        .iter()
        .find(|item| !item.viewed)
        .or_else(|| visible_files.first())
        .cloned();
    let season_hash = torrent.hash.clone();
    let episode_hash = torrent.hash.clone();
    let collection_torrent = torrent.clone();
    let later_torrent = torrent.clone();
    let year = metadata.as_ref().and_then(|item| item.year);
    let rating = metadata.as_ref().and_then(|item| item.rating);
    let overview = metadata.as_ref().and_then(|item| item.overview.clone());
    let genres = metadata
        .as_ref()
        .map(|item| item.genres.clone())
        .unwrap_or_default();
    rsx! {
        section { class: "screen detail-screen",
            section { class: "detail-hero",
                if let Some(poster) = poster.as_ref() {
                    div { class: "detail-art", style: "background-image:url('{poster}');" }
                }
                div { class: "detail-poster", {poster_art(poster.as_deref(), &title, 44)} }
                div { class: "detail-copy",
                    div { class: "meta-row",
                        span { class: "meta-pill", "BTIH · {short_hash(&torrent.hash)}" }
                        if let Some(year) = year { span { class: "meta-pill", "{year}" } }
                        if let Some(rating) = rating { span { class: "meta-pill accent", "★ {rating:.1}" } }
                        if media_type.as_deref() == Some("series") { span { class: "meta-pill", {language.pick("Сериал", "Series")} } }
                        for genre in genres.iter().take(3) { span { class: "meta-pill", "{genre}" } }
                    }
                    h1 { "{title}" }
                    if let Some(overview) = overview {
                        p { class: "detail-overview", "{overview}" }
                    } else {
                        p { class: "detail-overview", {language.pick("Локальное описание ещё не загружено. Обновите карточку в разделе управления.", "The local description has not been loaded yet. Refresh the title under management.")} }
                    }
                    div { class: "detail-actions",
                        if let Some(file) = next_file.clone() {
                            button { class: "button primary", disabled: busy, onclick: { let file = file.file.clone(); move |_| on_play.call((file.clone(), true, auto_next())) },
                                Icon { name: "play", size: 16 }
                                if media_type.as_deref() == Some("series") { if language == Language::Russian { "Смотреть серию" } else { "Play episode" } } else { {language.pick("Смотреть", "Watch")} }
                            }
                        }
                        button { class: if favorite { "icon-button active" } else { "icon-button" }, title: if favorite { language.pick("Убрать из избранного", "Remove from favorites") } else { language.pick("В избранное", "Add to favorites") }, onclick: move |_| on_collection.call((collection_torrent.clone(), CollectionList::Favorites, !favorite)),
                            Icon { name: "heart", size: 18 }
                        }
                        button { class: if watch_later { "icon-button active" } else { "icon-button" }, title: if watch_later { language.pick("Убрать из «Смотреть позже»", "Remove from watch later") } else { language.pick("Смотреть позже", "Watch later") }, onclick: move |_| on_collection.call((later_torrent.clone(), CollectionList::WatchLater, !watch_later)),
                            Icon { name: "clock", size: 18 }
                        }
                        button { class: "button ghost", onclick: move |event| on_back.call(event), Icon { name: "arrow-left", size: 16 } {language.pick("Назад", "Back")} }
                    }
                }
            }

            div { class: "detail-body",
                if !metadata_status.is_empty() { div { class: "app-notice", "{metadata_status}" } }
                if media_type.as_deref() == Some("series") && !season_progress.is_empty() {
                    div { class: "detail-section",
                        h2 { {language.pick("Сезоны", "Seasons")} }
                        div { class: "season-strip",
                            for (number, watched, total) in season_progress {
                                span { class: "season-pill",
                                    if number == 0 { {language.pick("Спецвыпуски", "Specials")} } else if language == Language::Russian { "Сезон {number}" } else { "Season {number}" }
                                    b { "{watched}/{total}" }
                                }
                            }
                        }
                    }
                }

                div { class: "detail-section",
                    div { class: "shelf-head",
                        h2 { {language.pick("Файлы", "Files")} }
                        if has_next_file {
                            label { class: "check",
                                input { r#type: "checkbox", checked: auto_next(), onchange: { let hash = season_hash.clone(); move |event| { auto_next.set(event.checked()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } } }
                                {language.pick("Автопереход", "Auto next")}
                            }
                        }
                    }
                    if !files_empty {
                        div { class: "detail-tools",
                            label { class: "input-wrap",
                                Icon { name: "search", size: 16 }
                                input { class: "text-input", value: "{file_filter}", placeholder: language.pick("Найти фильм или серию", "Find a film or episode"), oninput: move |event| file_filter.set(event.value()) }
                            }
                            if has_movies && has_episodes {
                                select { class: "select", value: "{file_kind}", onchange: move |event| { file_kind.set(event.value()); season.set("all".into()); episode.set("all".into()); },
                                    option { value: "all", {language.pick("Фильмы и серии", "Movies and episodes")} }
                                    option { value: "movies", {language.pick("Только фильмы", "Movies only")} }
                                    option { value: "episodes", {language.pick("Только серии", "Episodes only")} }
                                }
                            }
                            if seasons.len() > 1 {
                                select { class: "select", value: "{season}", onchange: { let hash = season_hash.clone(); move |event| { season.set(event.value()); episode.set("all".into()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } },
                                    option { value: "all", {language.pick("Все сезоны", "All seasons")} }
                                    for number in seasons { option { value: "{number}", if number == 0 { {language.pick("Спецвыпуски", "Specials")} } else if language == Language::Russian { "Сезон {number}" } else { "Season {number}" } } }
                                }
                            }
                            if !episodes.is_empty() {
                                select { class: "select", value: "{episode}", onchange: { let hash = episode_hash.clone(); move |event| { episode.set(event.value()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } },
                                    option { value: "all", {language.pick("Все серии", "All episodes")} }
                                    for number in episodes { option { value: "{number}", if language == Language::Russian { "Серия {number}" } else { "Episode {number}" } } }
                                }
                            }
                        }
                    }
                    if busy && files_empty {
                        div { class: "empty-state",
                            div { class: "empty-state-icon", Icon { name: "refresh", size: 28 } }
                            h2 { {language.pick("Получаем список файлов…", "Loading the file list…")} }
                            p { {language.pick("Подключаемся к раздаче и читаем метаданные.", "Connecting to the torrent and reading metadata.")} }
                        }
                    } else if files_empty {
                        div { class: "empty-state",
                            div { class: "empty-state-icon", Icon { name: "film", size: 28 } }
                            h2 { {language.pick("Видеофайлы пока недоступны", "Video files are not available yet")} }
                            p { {language.pick("Повторно откройте карточку через несколько секунд.", "Reopen this title in a few seconds.")} }
                        }
                    } else if visible_files.is_empty() {
                        div { class: "empty-state", h2 { {language.pick("Файлы не найдены", "No files match")} } }
                    } else {
                        div { class: "file-list",
                            for item in visible_files {
                                {
                                    let progress = progress_percent(item.position, item.duration);
                                    let size = format_bytes(item.file.length);
                                    let label = file_label(&item.file);
                                    let file = item.file.clone();
                                    let file_hash = torrent.hash.clone();
                                    rsx! {
                                        article { class: "file-row", key: "{item.file.id}",
                                            div { class: "file-index", "{item.file.id + 1}" }
                                            div { class: "file-copy",
                                                strong { title: "{item.file.path}", "{item.file.name}" }
                                                small { "{label}" if !size.is_empty() { " · {size}" } }
                                                if item.position > 0 {
                                                    div { class: "file-progress",
                                                        div { class: "progress-track", span { style: "width:{progress}%" } }
                                                        span { "{clock(item.position)} / {clock(item.duration)}" }
                                                    }
                                                } else if item.viewed {
                                                    small { {language.pick("Просмотрено", "Viewed")} }
                                                }
                                            }
                                            button { class: if item.viewed { "button secondary" } else { "button primary" }, disabled: busy, onclick: move |_| { let _ = &file_hash; on_play.call((file.clone(), true, auto_next())) },
                                                Icon { name: "play", size: 14 }
                                                if item.position > 0 { {language.pick("Продолжить", "Resume")} } else { {language.pick("Смотреть", "Play")} }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                details { class: "settings-card",
                    summary { class: "manage-summary",
                        span { {language.pick("Управление карточкой", "Manage title")} }
                        Icon { name: "settings", size: 16 }
                    }
                    div { class: "settings-grid",
                        label { class: "field-label", {language.pick("Название в медиатеке", "Library title")}
                            input { class: "text-input", value: "{title_edit}", oninput: move |event| title_edit.set(event.value()) }
                        }
                        div { class: "field-label", {language.pick("Тип", "Type")}
                            div { class: "chip-row",
                                button { class: if media_type.as_deref() == Some("movie") { "chip active" } else { "chip" }, onclick: move |_| on_media_type.call("movie".into()), {language.pick("Фильм", "Movie")} }
                                button { class: if media_type.as_deref() == Some("series") { "chip active" } else { "chip" }, onclick: move |_| on_media_type.call("series".into()), {language.pick("Сериал", "Series")} }
                            }
                        }
                    }
                    div { class: "heading-actions",
                        button { class: "button secondary", onclick: move |_| on_title.call(title_edit()), {language.pick("Сохранить название", "Save title")} }
                        button { class: "button secondary", disabled: busy, onclick: move |event| on_refresh_metadata.call(event), Icon { name: "refresh", size: 15 } {language.pick("Обновить описание и постер", "Refresh metadata and poster")} }
                    }
                    if confirm_remove() {
                        div { class: "app-notice error",
                            {language.pick("Убрать раздачу из медиатеки? История просмотра останется локально. ", "Remove this torrent from the library? Playback history will remain local. ")}
                            div { class: "notice-actions",
                                button { class: "button danger small", onclick: move |event| on_remove.call(event), {language.pick("Удалить", "Remove")} }
                                button { class: "button ghost small", onclick: move |_| confirm_remove.set(false), {language.pick("Отмена", "Cancel")} }
                            }
                        }
                    } else {
                        button { class: "button danger small", onclick: move |_| confirm_remove.set(true), Icon { name: "trash", size: 15 } {language.pick("Убрать из медиатеки", "Remove from library")} }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SettingsSection {
    General,
    Search,
    Remote,
    Maintenance,
    About,
}

#[component]
pub fn Settings(
    mut language: Signal<Language>,
    busy: Signal<bool>,
    server: Signal<ServerState>,
    cards: Signal<Vec<LibraryCard>>,
    recent: Signal<Vec<ContinueItem>>,
    metadata_status: Signal<String>,
    on_sync: EventHandler<MouseEvent>,
) -> Element {
    let initial = INITIAL_PREFERENCES
        .get()
        .cloned()
        .unwrap_or_else(default_preferences);
    let mut section = use_signal(|| SettingsSection::General);
    let cover_mode = use_context::<Signal<CoverMode>>();
    let mut torznab_url = use_signal(|| initial.torznab_url.clone());
    let mut torznab_api_key = use_signal(|| initial.torznab_api_key.clone());
    let mut close_to_tray = use_signal(|| initial.close_to_tray);
    let mut status = use_signal(String::new);
    let mut diagnostics = use_signal(Vec::<String>::new);
    let mut maintenance_busy = use_signal(|| false);
    let mut update_status = use_signal(String::new);
    let mut available_update = use_signal(|| None::<ReleaseUpdate>);
    let mut remote_info = use_signal(remote_control::info);
    let mut remote_message = use_signal(String::new);
    let window = dioxus::desktop::use_window();
    let brand_icon = png_data_uri(APP_ICON);

    let save = move |_| {
        let next = Preferences {
            language: language(),
            onboarding_complete: true,
            torznab_url: torznab_url(),
            torznab_api_key: torznab_api_key(),
            close_to_tray: close_to_tray(),
            cover_mode: cover_mode(),
        };
        status.set(
            match settings_path().and_then(|path| settings::save_preferences(&path, &next)) {
                Ok(()) => language()
                    .pick("Настройки сохранены", "Settings saved")
                    .into(),
                Err(error) => error,
            },
        );
    };
    let run_diagnostics = move |_| {
        if maintenance_busy() {
            return;
        }
        maintenance_busy.set(true);
        diagnostics.set(vec![language()
            .pick("Выполняем проверки…", "Running checks…")
            .into()]);
        spawn(async move {
            let result = tokio::task::spawn_blocking(system_diagnostics).await;
            diagnostics.set(match result {
                Ok(items) => items,
                Err(error) => vec![format!("✕ {error}")],
            });
            maintenance_busy.set(false);
        });
    };
    let create_backup = move |_| {
        if maintenance_busy() {
            return;
        }
        let Some(directory) = rfd::FileDialog::new()
            .set_title("Pirate Cinema backup")
            .pick_folder()
        else {
            return;
        };
        maintenance_busy.set(true);
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                create_local_backup(
                    &history_path()?,
                    settings_path().ok().as_deref(),
                    None,
                    &directory,
                )
            })
            .await;
            status.set(match result {
                Ok(Ok(path)) => format!(
                    "{} {}",
                    language().pick("Копия создана:", "Backup created:"),
                    path.display()
                ),
                Ok(Err(error)) => error,
                Err(error) => error.to_string(),
            });
            maintenance_busy.set(false);
        });
    };
    let check_update = move |_| {
        if maintenance_busy() {
            return;
        }
        maintenance_busy.set(true);
        update_status.set(
            language()
                .pick("Проверяем GitHub Releases…", "Checking GitHub Releases…")
                .into(),
        );
        spawn(async move {
            let result = tokio::task::spawn_blocking(|| check_rust_update(VERSION)).await;
            match result {
                Ok(Ok(Some(update))) => {
                    update_status.set(format!(
                        "{} {}",
                        language().pick("Доступна версия", "Version available"),
                        update.version
                    ));
                    available_update.set(Some(update));
                }
                Ok(Ok(None)) => update_status.set(
                    language()
                        .pick("Установлена актуальная версия", "You are up to date")
                        .into(),
                ),
                Ok(Err(error)) => update_status.set(error),
                Err(error) => update_status.set(error.to_string()),
            }
            maintenance_busy.set(false);
        });
    };
    let install_update = move |_| {
        let Some(update) = available_update() else {
            return;
        };
        maintenance_busy.set(true);
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let installer = download_rust_update(&update)?;
                launch_update_installer(&installer)?;
                Ok::<_, String>(installer)
            })
            .await;
            update_status.set(match result {
                Ok(Ok(path)) => format!(
                    "{} {}",
                    language().pick("Установщик запущен:", "Installer started:"),
                    path.display()
                ),
                Ok(Err(error)) => error,
                Err(error) => error.to_string(),
            });
            maintenance_busy.set(false);
        });
    };
    let exit_fullscreen = move |_| {
        window.set_fullscreen(false);
        spawn(async move {
            let mut eval = document::eval(
                r#"const art=window.__pirateCinemaArt;if(art){art.fullscreen=false;art.fullscreenWeb=false;}if(document.fullscreenElement){await document.exitFullscreen().catch(()=>{});}dioxus.send(true);"#,
            );
            let _ = eval.recv::<bool>().await;
        });
    };
    let engine_online = !server.read().version.is_empty();
    let engine_version = server.read().version.clone();
    let torrent_count = server.read().torrents.len();
    let card_count = cards.read().len();
    let recent_count = recent.read().len();
    let cover_preview = cards
        .read()
        .first()
        .map(|card| card_cover(card, cover_mode()));
    let current_section = section();
    rsx! {
        section { class: "screen settings-screen",
            div { class: "screen-heading",
                div {
                    h1 { {language().pick("Настройки", "Settings")} }
                    p { "Pirate Cinema · {VERSION}. " {language().pick("Всё остаётся на этом компьютере.", "Everything stays on this computer.")} }
                }
                div { class: "heading-actions",
                    button { class: "button primary", onclick: save, {language().pick("Сохранить", "Save")} }
                }
            }
            if !status().is_empty() { div { class: "app-notice", "{status}" } }
            div { class: "settings-layout",
                nav { class: "settings-nav", "aria-label": language().pick("Разделы настроек", "Settings sections"),
                    button { class: if current_section == SettingsSection::General { "active" } else { "" }, onclick: move |_| section.set(SettingsSection::General), Icon { name: "monitor", size: 17 } span { {language().pick("Общие", "General")} } }
                    button { class: if current_section == SettingsSection::Search { "active" } else { "" }, onclick: move |_| section.set(SettingsSection::Search), Icon { name: "search", size: 17 } span { {language().pick("Поиск", "Search")} } }
                    button { class: if current_section == SettingsSection::Remote { "active" } else { "" }, onclick: move |_| section.set(SettingsSection::Remote), Icon { name: "wifi", size: 17 } span { {language().pick("Веб-пульт", "Web remote")} } }
                    button { class: if current_section == SettingsSection::Maintenance { "active" } else { "" }, onclick: move |_| section.set(SettingsSection::Maintenance), Icon { name: "database", size: 17 } span { {language().pick("Обслуживание", "Maintenance")} } }
                    button { class: if current_section == SettingsSection::About { "active" } else { "" }, onclick: move |_| section.set(SettingsSection::About), Icon { name: "info", size: 17 } span { {language().pick("О приложении", "About")} } }
                }
                div { class: "settings-panel",
                    match current_section {
                        SettingsSection::General => rsx! {
                            section { class: "settings-card",
                                h2 { {language().pick("Интерфейс и окно", "Interface and window")} }
                                div { class: "settings-grid",
                                    label { class: "field-label", {language().pick("Язык", "Language")}
                                        select { class: "select", value: if language() == Language::English { "en" } else { "ru" }, onchange: move |event| language.set(if event.value() == "en" { Language::English } else { Language::Russian }),
                                            option { value: "ru", "Русский" }
                                            option { value: "en", "English" }
                                        }
                                    }
                                    label { class: "field-label", {language().pick("При закрытии", "When closing")}
                                        select { class: "select", value: if close_to_tray() { "tray" } else { "exit" }, onchange: move |event| close_to_tray.set(event.value() == "tray"),
                                            option { value: "tray", {language().pick("Сворачивать в трей", "Minimize to tray")} }
                                            option { value: "exit", {language().pick("Закрывать приложение", "Exit application")} }
                                        }
                                    }
                                }
                                div { class: "meta-row",
                                    span { class: "meta-pill", "ArtPlayer" }
                                    span { class: "meta-pill", "hls.js" }
                                    span { class: "meta-pill", "librqbit" }
                                    span { class: "meta-pill", "SQLite" }
                                }
                                p { class: "hint", {language().pick("Плеер открывается внутри приложения. Полноэкранный режим можно завершить здесь, если окно осталось на другом мониторе.", "Playback runs inside the app. End fullscreen here if the window stayed on another display.")} }
                                button { class: "button secondary", onclick: exit_fullscreen, {language().pick("Выйти из полноэкранного режима", "Exit fullscreen")} }
                            }
                            section { class: "settings-card cover-settings",
                                div { class: "cover-settings-head",
                                    div {
                                        h2 { {language().pick("Обложки", "Cover art")} }
                                        p { class: "hint", {language().pick("Чистые обложки собираются на этом компьютере из названия, года, жанров и хеша: без загрузки изображений и без сети. Смена стиля применяется сразу и не удаляет кэш постеров.", "Clean covers are composed on this machine from the title, year, genres and hash: no uploads and no network. Changing the style applies immediately and never deletes the poster cache.")} }
                                    }
                                    if let Some(preview) = cover_preview.as_ref() {
                                        img { class: "cover-mode-preview", src: "{preview}", alt: language().pick("Пример локальной обложки", "Generated cover preview") }
                                    }
                                }
                                div { class: "cover-mode-grid", role: "group", "aria-label": language().pick("Стиль обложек", "Cover style"),
                                    button {
                                        class: if cover_mode() == CoverMode::Auto { "cover-mode-option active" } else { "cover-mode-option" },
                                        "aria-pressed": if cover_mode() == CoverMode::Auto { "true" } else { "false" },
                                        onclick: move |_| select_cover_mode(cover_mode, status, language(), CoverMode::Auto),
                                        Icon { name: "sparkles", size: 18 }
                                        span { strong { {language().pick("Авто", "Auto")} } small { {language().pick("Чистая обложка, когда есть описание; иначе постер из кэша.", "Clean art when metadata exists; cached poster otherwise.")} } }
                                        if cover_mode() == CoverMode::Auto { Icon { name: "check", size: 15 } }
                                    }
                                    button {
                                        class: if cover_mode() == CoverMode::Generated { "cover-mode-option active" } else { "cover-mode-option" },
                                        "aria-pressed": if cover_mode() == CoverMode::Generated { "true" } else { "false" },
                                        onclick: move |_| select_cover_mode(cover_mode, status, language(), CoverMode::Generated),
                                        Icon { name: "film", size: 18 }
                                        span { strong { {language().pick("Локальные", "Local")} } small { {language().pick("Всегда типографическая обложка из локальных данных.", "Always a typographic cover from local data.")} } }
                                        if cover_mode() == CoverMode::Generated { Icon { name: "check", size: 15 } }
                                    }
                                    button {
                                        class: if cover_mode() == CoverMode::Poster { "cover-mode-option active" } else { "cover-mode-option" },
                                        "aria-pressed": if cover_mode() == CoverMode::Poster { "true" } else { "false" },
                                        onclick: move |_| select_cover_mode(cover_mode, status, language(), CoverMode::Poster),
                                        Icon { name: "download", size: 18 }
                                        span { strong { {language().pick("Постеры", "Posters")} } small { {language().pick("Сначала постер провайдера; локальная обложка — запасной вариант.", "Provider poster first; local art as the fallback.")} } }
                                        if cover_mode() == CoverMode::Poster { Icon { name: "check", size: 15 } }
                                    }
                                }
                            }
                            section { class: "settings-card",
                                h2 { {language().pick("Локальность", "Local by default")} }
                                p { class: "privacy-note", {language().pick("Медиатека, история просмотра, постеры и настройки остаются в локальной папке профиля. Приложение не отправляет их на сервер.", "Library, viewing history, posters, and settings stay in the local profile folder. The app does not upload them to a server.")} }
                            }
                        },
                        SettingsSection::Search => rsx! {
                            section { class: "settings-card",
                                h2 { {language().pick("Torznab / Jackett", "Torznab / Jackett")} }
                                p { class: "hint", {language().pick("Поиск идёт по настроенным источникам. Ключ хранится только в локальном settings.json.", "Search runs across the sources you configure. The key is stored only in the local settings.json.")} }
                                div { class: "settings-grid",
                                    label { class: "field-label", {language().pick("Адрес Jackett/Torznab", "Jackett/Torznab URL")}
                                        input { class: "text-input", value: "{torznab_url}", placeholder: "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab", oninput: move |event| torznab_url.set(event.value()) }
                                    }
                                    label { class: "field-label", {language().pick("API ключ", "API key")}
                                        input { class: "text-input", r#type: "password", value: "{torznab_api_key}", oninput: move |event| torznab_api_key.set(event.value()) }
                                    }
                                }
                                button { class: "button primary", onclick: save, {language().pick("Сохранить настройки поиска", "Save search settings")} }
                            }
                        },
                        SettingsSection::Remote => rsx! {
                            section { class: "settings-card",
                                h2 { {language().pick("Веб-пульт", "Web remote")} }
                                p { class: "hint", {language().pick("Управляет только активным плеером в локальной сети. Телефон и компьютер должны быть в одной Wi-Fi сети.", "Controls only the active player on the local network. The phone and computer must share a Wi-Fi network.")} }
                                if let Some(info) = remote_info() {
                                    div { class: "remote-panel",
                                        img { class: "remote-qr", src: "{info.qr_data_uri}", alt: language().pick("QR-код веб-пульта", "Web remote QR code") }
                                        div { class: "remote-copy",
                                            label { class: "field-label", {language().pick("Адрес для телефона", "Phone URL")}
                                                input { class: "text-input", readonly: true, value: "{info.url}" }
                                            }
                                            p { class: "hint", {language().pick("Отсканируйте QR-код или откройте адрес вручную. Страницу можно добавить на главный экран телефона.", "Scan the QR code or open the address manually. The page can be added to the phone home screen.")} }
                                            button { class: "button secondary", onclick: move |_| { remote_control::stop(); remote_info.set(None); remote_message.set(language().pick("Веб-пульт выключен", "Web remote stopped").into()); }, {language().pick("Выключить пульт", "Stop remote")} }
                                        }
                                    }
                                } else {
                                    button { class: "button primary", onclick: move |_| match remote_control::start() {
                                        Ok(info) => { remote_info.set(Some(info)); remote_message.set(language().pick("Откройте адрес на телефоне в той же Wi-Fi сети", "Open the address on a phone on the same Wi-Fi network").into()); }
                                        Err(error) => remote_message.set(error),
                                    }, Icon { name: "wifi", size: 16 } {language().pick("Включить веб-пульт", "Start web remote")} }
                                }
                                if !remote_message().is_empty() { div { class: "app-notice", "{remote_message}" } }
                            }
                        },
                        SettingsSection::Maintenance => rsx! {
                            section { class: "settings-card",
                                h2 { {language().pick("Медиатека", "Library")} }
                                div { class: "settings-facts",
                                    article { span { {language().pick("Карточек", "Titles")} } strong { "{card_count}" } small { {language().pick("Локальная SQLite", "Local SQLite")} } }
                                    article { span { {language().pick("В истории", "In history")} } strong { "{recent_count}" } small { {language().pick("Продолжить просмотр", "Continue watching")} } }
                                    article { span { {language().pick("Торрентов", "Torrents")} } strong { "{torrent_count}" } small { "librqbit" } }
                                }
                                div { class: "heading-actions",
                                    button { class: "button primary", disabled: busy(), onclick: move |event| on_sync.call(event), Icon { name: "refresh", size: 15 } {language().pick("Обновить описания и постеры", "Refresh descriptions and posters")} }
                                    button { class: "button secondary", disabled: maintenance_busy(), onclick: create_backup, Icon { name: "folder", size: 15 } {language().pick("Создать резервную копию", "Create backup")} }
                                }
                                if !metadata_status().is_empty() { div { class: "app-notice", "{metadata_status()}" } }
                            }
                            section { class: "settings-card",
                                h2 { {language().pick("Диагностика и обновления", "Diagnostics and updates")} }
                                div { class: "heading-actions",
                                    button { class: "button secondary", disabled: maintenance_busy(), onclick: run_diagnostics, Icon { name: "database", size: 15 } {language().pick("Диагностика", "Diagnostics")} }
                                    button { class: "button secondary", disabled: maintenance_busy(), onclick: check_update, Icon { name: "download", size: 15 } {language().pick("Проверить обновления", "Check updates")} }
                                }
                                if !update_status().is_empty() {
                                    div { class: "app-notice",
                                        span { "{update_status()}" }
                                        if let Some(update) = available_update() {
                                            button { class: "button primary small", disabled: maintenance_busy(), onclick: install_update, {language().pick("Установить", "Install")} "{update.version}" }
                                        }
                                    }
                                }
                                if !diagnostics().is_empty() { div { class: "diagnostics", for item in diagnostics() { div { "{item}" } } } }
                            }
                        },
                        SettingsSection::About => rsx! {
                            section { class: "settings-card",
                                div { class: "about-brand",
                                    img { src: "{brand_icon}", alt: "" }
                                    div { strong { "Pirate Cinema" } small { "Desktop {VERSION} · Rust · Dioxus" } }
                                }
                                p { class: "privacy-note", {language().pick("Локальная медиатека с встроенным торрент-движком, HLS-мостом и ArtPlayer. Без аккаунта, облачной библиотеки и телеметрии.", "A local library with an embedded torrent engine, HLS bridge, and ArtPlayer. No account, cloud library, or telemetry.")} }
                                div { class: "meta-row",
                                    span { class: "meta-pill", "librqbit {engine_version}" }
                                    span { class: if engine_online { "meta-pill accent" } else { "meta-pill" }, if engine_online { {language().pick("Движок работает", "Engine online")} } else { {language().pick("Движок недоступен", "Engine offline")} } }
                                }
                            }
                            section { class: "settings-card",
                                h2 { {language().pick("Компоненты", "Components")} }
                                div { class: "settings-facts",
                                    article { span { "Engine" } strong { "librqbit" } small { {language().pick("Встроенный", "Embedded")} } }
                                    article { span { "Player" } strong { "ArtPlayer" } small { "hls.js · FFmpeg" } }
                                    article { span { "Storage" } strong { "SQLite" } small { {language().pick("История и метаданные", "History and metadata")} } }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
