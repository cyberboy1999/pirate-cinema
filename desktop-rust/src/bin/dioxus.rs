#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use base64::Engine;
use dioxus::prelude::*;
use pirate_cinema_core::history::{
    create_local_backup, HistoryStore, MediaMetadata, PlaybackPreferences,
};
use pirate_cinema_core::metadata::{self, Movie};
use pirate_cinema_core::migration::{legacy_profile_paths, migrate_legacy_profiles};
use pirate_cinema_core::mpv::{self, MpvEvent, MpvSession};
use pirate_cinema_core::settings::{self, Language, PlayerType, Preferences};
use pirate_cinema_core::torrserver_process::{
    bundled_executable as bundled_torrserver, default_data_dir, TorrServerProcess,
};
use pirate_cinema_core::{
    add_magnet, check_rust_update, configure_torznab, download_rust_update, magnet_info_hash,
    magnet_title, probe_stream, read_torrserver, remove_torrent, search_all_sources, stream_url,
    torrent_video_files, ReleaseUpdate, SearchResult, Torrent, VideoFile, DEFAULT_TORRSERVER_URL,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

static STARTUP_MAGNET: OnceLock<String> = OnceLock::new();
static INITIAL_PREFERENCES: OnceLock<Preferences> = OnceLock::new();
static TORRSERVER_PROCESS: OnceLock<Mutex<Option<TorrServerProcess>>> = OnceLock::new();
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static METADATA_SYNC_ACTIVE: AtomicBool = AtomicBool::new(false);
static PLAYBACK_VISIBLE: AtomicBool = AtomicBool::new(true);
static STARTUP_MIGRATION_ERROR: OnceLock<String> = OnceLock::new();
static STARTUP_TORRSERVER_ERROR: OnceLock<String> = OnceLock::new();
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ICON: &[u8] = include_bytes!("../../../public/favicon.png");
const HOME_ICON: &[u8] = include_bytes!(
    "../../assets/icons/building_business_home_house_mobile_navigation_office_icon_123210.png"
);
const LIBRARY_ICON: &[u8] =
    include_bytes!("../../assets/icons/video-camera_icon-icons.com_53843.png");
const SETTINGS_ICON: &[u8] = include_bytes!(
    "../../assets/icons/3643771-configuration-configure-gear-set-setting_113449.png"
);
const ONLINE_ICON: &[u8] = include_bytes!("../../assets/icons/online_4158.png");
const HLS_JS: &str = include_str!("../../assets/hls.min.js");
const ARTPLAYER_JS: &str = include_str!("../../assets/artplayer.js");
const PLAYER_STATE_JS: &str = include_str!("../../assets/player-state.js");

const CSS: &str = r#"
:root { color-scheme: dark; font-family: Inter, "Segoe UI", sans-serif; font-size:17px; background: #050505; color: #f2f2f2; }
* { box-sizing: border-box; }
body { margin: 0; min-width: 760px; background: #050505; }
button, input { font: inherit; }
button { color: inherit; cursor: pointer; }
.shell { min-height: 100vh; display: grid; grid-template-columns: 230px minmax(0, 1fr); }
.welcome-shell { min-height: 100vh; display: grid; place-items: center; padding: 24px; background: radial-gradient(circle at 50% 20%, #1d1d1d, #050505 58%); }
.welcome-card { width: min(520px, 100%); display: grid; gap: 15px; padding: 34px; border: 1px solid #292929; border-radius: 20px; background: #0d0d0d; box-shadow: 0 22px 80px #000; }
.welcome-card label { display: grid; gap: 8px; color: #aaa; }
.welcome-card input { padding: 13px 14px; border: 1px solid #292929; border-radius: 10px; background: #080808; color: white; }
.welcome-action { min-height: 46px; }
.sidebar { position: sticky; top: 0; height: 100vh; padding: 28px 14px 22px; border-right: 1px solid #202020; display: flex; flex-direction: column; background: #080808; }
.brand { margin: 0 4px 34px; display:flex; align-items:center; gap:10px; font-size: 20px; font-weight: 800; letter-spacing: -.7px; white-space:nowrap; }
.brand img { width:30px; height:30px; padding:3px; border-radius:7px; background:#ddd; object-fit:contain; }
.nav { display: grid; gap: 8px; }
.rail-bottom{margin-top:auto;display:grid;gap:8px}
.nav button,.rail-bottom button { width: 100%; min-height:52px; padding: 0 14px; display:flex; align-items:center; gap:13px; border: 0; border-radius: 10px; background: transparent; color: #969696; font-weight:600; text-align: left; transition:background .15s,color .15s; }
.nav button:hover, .nav button.active,.rail-bottom button:hover,.rail-bottom button.active { color: #f2f2f2; background: #1a1a1a; }
.nav-icon { width:24px; height:24px; flex:0 0 auto; object-fit:contain; filter:invert(1); opacity:.62; }
.nav button:hover .nav-icon,.nav button.active .nav-icon,.rail-bottom button:hover .nav-icon,.rail-bottom button.active .nav-icon{opacity:.95}
.search-icon { width:20px; height:20px; border:2px solid currentColor; border-radius:50%; position:relative; opacity:.66; }
.search-icon:after { content:""; position:absolute; width:7px; height:2px; right:-5px; bottom:-2px; border-radius:2px; background:currentColor; transform:rotate(45deg); }
.version { padding: 14px 10px 0; color: #666; font-size: 12px; }
.content { min-width: 0; padding: 30px 32px 52px; }
.topbar { display: grid; grid-template-columns: minmax(360px, 920px) minmax(180px,1fr); gap: 22px; align-items: center; }
.search { height: 54px; display: flex; gap: 10px; padding: 5px 6px 5px 18px; border: 1px solid #242424; border-radius: 13px; background: #101010; }
.search>.search-icon{margin:auto 5px auto 0;width:17px;height:17px;flex:0 0 auto}
.search input { min-width: 0; flex: 1; border: 0; outline: 0; color: white; background: transparent; }
.search button, .primary { border: 0; border-radius: 9px; padding: 0 18px; background: #ededed; color: #080808; font-weight: 700; transition:background .15s,transform .15s; }
.search button:hover,.primary:hover{background:#fff}.search button:active,.primary:active{transform:translateY(1px)}
.status { justify-self:end; display:flex;align-items:center;gap:8px;white-space: nowrap; color: #73d99a; font-size: 14px; }
.status img{width:21px;height:21px;object-fit:contain;filter:invert(78%) sepia(29%) saturate(725%) hue-rotate(91deg);}
.status.offline{color:#ef6464}
.status.offline img{filter:invert(48%) sepia(92%) saturate(1088%) hue-rotate(323deg) brightness(100%)}
.page { margin-top: 48px; }
.eyebrow { color: #747474; font-size: 11px; font-weight: 700; letter-spacing: 1.5px; text-transform: uppercase; }
h1 { margin: 8px 0 10px; font-size: 36px; letter-spacing: -1.2px; }
.lead { max-width: 760px; margin: 0 0 30px; color: #8b8b8b; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(168px, 188px)); gap: 28px 18px; align-items:start; }
.card { min-width: 0; border: 0; padding: 0; background: transparent; text-align: left; }
.poster { aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; display: grid; place-items: center; background: linear-gradient(145deg, #202020, #101010); color: #5f5f5f; font-size: 42px; transition:transform .16s,box-shadow .16s; }
.poster img { width: 100%; height: 100%; object-fit: cover; }
.card:hover .poster{transform:translateY(-3px);box-shadow:0 14px 28px rgba(0,0,0,.45)}
.card strong { display: block; margin-top: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size:15px;line-height:1.35; }
.card small { display: block; margin-top: 6px; color: #888; font-size:13px; }
.card small.viewed { color: #71cc91; }
.card-open{display:block;width:100%;padding:0;border:0;background:transparent;color:inherit;text-align:left}
.home-recent{display:grid;grid-template-columns:repeat(auto-fill,minmax(172px,1fr));gap:22px 18px;max-width:1320px}
.empty, .panel { border: 1px solid #242424; border-radius: 16px; padding: 26px; background: #101010; color: #999; }
.results { display: grid; gap: 8px; }
.search-layout{display:grid;grid-template-columns:240px minmax(0,1fr);gap:34px;align-items:start}.search-feature{position:sticky;top:24px}.search-feature .poster{width:100%;margin-bottom:18px}.search-feature h1{font-size:32px;margin:0 0 8px}.search-feature .overview{font-size:14px;line-height:1.55}.search-results h2{margin:0 0 18px;font-size:25px}
.result { display: grid; grid-template-columns: minmax(0, 1fr) 90px 70px auto auto; gap: 10px; align-items: center; padding: 14px 16px; border: 1px solid #222; border-radius: 12px; background: #0d0d0d; }
.result strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.result span, .result small { color: #888; }
.secondary, .result button, .file button { min-height: 42px; border: 1px solid #303030; border-radius: 9px; padding: 0 16px; background: #191919; font-weight:600; transition:background .15s,border-color .15s; }
.secondary:hover,.result button:hover,.file button:hover{background:#242424;border-color:#555}
.result button.result-action{background:#fff;color:#080808;border-color:#fff}
.result button.result-action:hover{background:#e5e5e5;border-color:#e5e5e5}
.file button.launch-button{background:#fff;color:#080808;border-color:#fff}
.file button.launch-button:hover{background:#e5e5e5;border-color:#e5e5e5}
button:disabled{cursor:not-allowed;opacity:.48;transform:none!important}
button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid #ddd;outline-offset:2px}
.detail-layout { display: grid; grid-template-columns: 210px minmax(0, 1fr); gap: 28px; }
.detail-cover { width: 210px; aspect-ratio: 2 / 3; border-radius: 14px; object-fit: cover; background: #171717; }
.detail-head { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 18px; align-items: end; }
.overview { max-width: 780px; color: #aaa; line-height: 1.65; }
.files { display: grid; gap: 9px; margin-top: 28px; }
.file { display: grid; grid-template-columns: minmax(220px, 1fr) auto; gap: 10px; align-items: center; padding: 14px 16px; border: 1px solid #222; border-radius: 12px; background: #0d0d0d; }
.file strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.settings { width:min(820px,100%); display: grid; gap: 18px; padding-bottom:60px; }
.settings>h1{margin-bottom:0}.settings>.lead{margin-bottom:8px}
.settings-card{display:grid;gap:16px;padding:22px;border:1px solid #292929;border-radius:15px;background:#0d0d0d}.settings-card h2{margin:0;font-size:20px}.settings-card .hint{margin:-7px 0 0;color:#777;font-size:13px;line-height:1.45}
.settings label { display: grid; gap: 8px; color: #aaa; }
.settings input, .settings select { min-height:46px;padding: 13px 14px; border: 1px solid #292929; border-radius: 10px; background: #0b0b0b; color: white; }
.settings label.auto-next{display:flex;align-items:center;gap:10px}.settings .auto-next input[type="checkbox"]{width:18px;height:18px;min-height:0;margin:0;padding:0;flex:0 0 18px;accent-color:#e8e8e8}
.setting-row{display:grid;grid-template-columns:180px minmax(0,1fr);gap:18px;align-items:center}.setting-row>span{color:#aaa;font-size:14px;text-transform:uppercase;letter-spacing:.6px}.setting-row label{display:contents}.choice-row{display:grid;grid-template-columns:1fr 1fr;gap:10px}.choice-row button.selected{border-color:#777;background:#282828;color:#fff}.settings-actions { display:grid;grid-template-columns:1fr 1fr;gap:10px}.settings-card .primary{min-height:46px}.settings-output{display:grid;gap:10px}.server-summary{grid-template-columns:52px minmax(0,1fr);align-items:center}.server-summary img{width:34px;height:34px;object-fit:contain;filter:brightness(0) invert(1);opacity:.82}.server-summary strong{display:block;font-size:17px}.server-summary small{display:block;margin-top:4px;color:#8a8a8a}.settings-facts{display:grid;grid-template-columns:repeat(3,1fr);gap:10px}.settings-facts article{padding:15px;border:1px solid #282828;border-radius:11px;background:#111}.settings-facts span,.settings-facts small{display:block;color:#777;font-size:12px}.settings-facts strong{display:block;margin:6px 0;font-size:15px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.stacked-options{display:grid;gap:10px}
.filters { display: flex; flex-wrap: wrap; gap: 10px; margin: 0 0 24px; }
.filters input, .filters select { min-height: 42px; padding: 0 13px; border: 1px solid #292929; border-radius: 10px; background: #0b0b0b; color: white; }
.filters input { min-width: 240px; }
.notice { margin: 22px 0; padding: 13px 15px; border-radius: 10px; background: #191919; color: #aaa; }
.player-strip { margin: 14px 0 0; display: grid; grid-template-columns: minmax(0, 1fr) repeat(3, auto); gap: 8px; align-items: center; padding: 10px; border: 1px solid #242424; border-radius: 12px; background: #0d0d0d; }
.player-strip span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.player-strip button { min-height: 34px; border: 1px solid #303030; border-radius: 8px; background: #191919; }
.web-player{margin-top:14px;padding:12px;border:1px solid #292929;border-radius:12px;background:#090909}.web-player header{display:flex;align-items:center;gap:10px;margin-bottom:10px}.web-player header strong{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.artplayer-host{width:100%;height:min(72vh,760px);min-height:420px;border-radius:9px;overflow:hidden;background:#000}.web-player.mini{position:fixed;z-index:100;right:20px;bottom:20px;width:min(420px,calc(100vw - 40px));margin:0;box-shadow:0 14px 48px #000b}.web-player.mini .artplayer-host{height:220px;min-height:0}.web-player.mini .pc-player-topbar{display:none}.art-video-player{--art-theme:#287cff}.pc-player-topbar{position:absolute;z-index:80;top:12px;left:12px;display:flex;gap:6px;max-width:calc(100% - 24px);padding:5px;border:1px solid #ffffff26;border-radius:9px;background:#090909d9;backdrop-filter:blur(8px);opacity:.42;transition:opacity .18s}.art-video-player:hover .pc-player-topbar,.pc-player-topbar:focus-within{opacity:1}.pc-player-topbar select{width:auto;min-width:96px;max-width:190px;height:32px;padding:0 28px 0 10px;border:1px solid #ffffff29;border-radius:7px;background:#181818;color:#fff;font:600 13px system-ui;cursor:pointer}.pc-player-topbar .pc-season{min-width:105px}.pc-player-topbar .pc-episode{min-width:112px}.pc-player-topbar .pc-audio{min-width:130px;max-width:240px}
.pc-next-episode{position:absolute;z-index:80;right:18px;bottom:74px;display:none;padding:10px 16px;border:0;border-radius:8px;background:#f2f2f2;color:#111;font:700 14px system-ui;cursor:pointer;box-shadow:0 4px 18px #0009}.pc-next-episode:hover{background:#fff}.pc-next-episode:focus-visible{outline:2px solid #287cff;outline-offset:2px}
.pc-auto-next{display:flex;align-items:center;white-space:nowrap;font:600 13px system-ui;gap:4px;padding:0 6px}.pc-auto-next input{width:16px;height:16px;accent-color:#287cff}.pc-stream-status{position:absolute;z-index:75;bottom:80px;left:18px;max-width:65%;padding:8px 12px;background:#111d;border-radius:8px;font:14px system-ui}.pc-stream-status[hidden]{display:none}.pc-stream-status button{margin-left:10px;padding:5px 9px;background:#fff;color:#111;border:0;border-radius:5px}
.season-progress{display:flex;flex-wrap:wrap;gap:8px;margin:12px 0 18px}.season-progress span{padding:8px 11px;border:1px solid #292929;border-radius:9px;background:#101010;color:#aaa;font-size:13px}
@media (max-width: 900px) { .shell { grid-template-columns: 78px 1fr; } .brand { margin-inline:auto;font-size: 0; } .brand img{width:34px;height:34px}.nav button { justify-content:center;overflow: hidden; white-space: nowrap; } .nav button span { display: none; } .content { padding: 24px 22px; } .topbar{grid-template-columns:1fr}.status{justify-self:start}.search-layout{grid-template-columns:1fr}.search-feature{position:static;display:grid;grid-template-columns:130px 1fr;gap:18px}.setting-row{grid-template-columns:1fr;gap:7px}.settings-actions,.choice-row,.settings-facts{grid-template-columns:1fr}.file { grid-template-columns: 1fr 1fr; } .file > div { grid-column: 1 / -1; } }
"#;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Search,
    Library,
    Detail,
    Player,
    Settings,
}

#[derive(Clone, PartialEq)]
struct ServerState {
    version: String,
    torrents: Vec<Torrent>,
    error: String,
}

#[derive(Clone, PartialEq)]
struct LibraryCard {
    torrent: Torrent,
    metadata: Option<MediaMetadata>,
    media_type: Option<String>,
    viewed: bool,
    poster: Option<String>,
}

#[derive(Clone, PartialEq)]
struct PlayableFile {
    file: VideoFile,
    position: i64,
    duration: i64,
    viewed: bool,
}

#[derive(Clone, PartialEq)]
struct ContinueItem {
    torrent: Torrent,
    title: String,
    file_name: String,
    position: i64,
    duration: i64,
}

#[derive(Clone, PartialEq)]
struct PendingPlayback {
    torrent: Torrent,
    file: VideoFile,
    resume: bool,
    queue: Vec<VideoFile>,
    auto_next: bool,
    force_mpv: bool,
}

#[derive(Clone, PartialEq)]
struct WebPlayback {
    torrent: Torrent,
    file: VideoFile,
    queue: Vec<VideoFile>,
    auto_next: bool,
    url: String,
    probe_url: String,
    heartbeat_url: String,
    position: i64,
}

enum PlaybackLaunch {
    Mpv(MpvSession),
    Web(WebPlayback),
    External,
}

struct OwnedPlayer {
    session: MpvSession,
    torrent: Torrent,
    file: VideoFile,
    queue: Vec<VideoFile>,
    auto_next: bool,
}

#[derive(Clone)]
struct TrayState(tray_icon::TrayIcon);

fn main() {
    if let Some(magnet) = std::env::args().skip(1).find(|argument| {
        argument
            .trim_matches(['\'', '"'])
            .to_ascii_lowercase()
            .starts_with("magnet:?")
    }) {
        let _ = STARTUP_MAGNET.set(magnet.trim_matches(['\'', '"']).to_owned());
    }
    if let Ok(data_dir) = default_data_dir() {
        if let Some(root) = data_dir.parent() {
            let (electron, prototype) = legacy_profile_paths();
            if let Err(error) =
                migrate_legacy_profiles(root, electron.as_deref(), prototype.as_deref())
            {
                let _ = STARTUP_MIGRATION_ERROR.set(format!(
                    "Не удалось перенести данные Electron 0.5.8: {error}"
                ));
            }
        }
    }
    let mut preferences = settings_path()
        .ok()
        .and_then(|path| settings::load_preferences(&path).ok())
        .unwrap_or_else(default_preferences);
    let endpoint =
        std::env::var("TORRSERVER_URL").unwrap_or_else(|_| preferences.torrserver_url.clone());
    preferences.torrserver_url = endpoint.clone();
    let close_to_tray = preferences.close_to_tray;
    let _ = INITIAL_PREFERENCES.set(preferences);
    let _ = TORRSERVER_PROCESS.set(Mutex::new(None));
    std::thread::spawn(move || {
        let result = bundled_torrserver()
            .and_then(|executable| default_data_dir().map(|data_dir| (executable, data_dir)))
            .and_then(|(executable, data_dir)| {
                TorrServerProcess::connect_or_start(&endpoint, &executable, &data_dir)
            });
        match result {
            Ok(process) => {
                if SHUTDOWN_REQUESTED.load(Ordering::Acquire) {
                    drop(process);
                    return;
                }
                if let Some(state) = TORRSERVER_PROCESS.get() {
                    if let Ok(mut owned) = state.lock() {
                        *owned = Some(process);
                    }
                }
            }
            Err(error) => {
                let _ = STARTUP_TORRSERVER_ERROR.set(error);
            }
        }
    });
    let window_icon =
        dioxus::desktop::icon_from_memory::<dioxus::desktop::tao::window::Icon>(APP_ICON).ok();
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_menu(None)
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title(format!("Pirate Cinema {VERSION} — локальная медиатека"))
                        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1440.0, 900.0))
                        .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(
                            1024.0, 700.0,
                        ))
                        .with_window_icon(window_icon),
                )
                .with_close_behaviour(if close_to_tray {
                    dioxus::desktop::WindowCloseBehaviour::WindowHides
                } else {
                    dioxus::desktop::WindowCloseBehaviour::WindowCloses
                }),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    let _shutdown_guard = use_hook(|| Arc::new(ShutdownGuard));
    let desktop = dioxus::desktop::use_window();
    let brand_icon = png_data_uri(APP_ICON);
    let home_icon = png_data_uri(HOME_ICON);
    let library_icon = png_data_uri(LIBRARY_ICON);
    let settings_icon = png_data_uri(SETTINGS_ICON);
    let online_icon = png_data_uri(ONLINE_ICON);
    let tray = use_hook(|| {
        let menu = tray_icon::menu::Menu::new();
        let open = tray_icon::menu::MenuItem::with_id("open", "Открыть Pirate Cinema", true, None);
        let exit = tray_icon::menu::MenuItem::with_id("exit", "Выход", true, None);
        menu.append_items(&[&open, &exit]).ok()?;
        let decoded = image::load_from_memory(APP_ICON).ok()?.into_rgba8();
        let (width, height) = decoded.dimensions();
        let icon = tray_icon::Icon::from_rgba(decoded.into_raw(), width, height).ok()?;
        Some(TrayState(
            tray_icon::TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("Pirate Cinema")
                .with_icon(icon)
                .build()
                .ok()?,
        ))
    });
    let has_tray = tray.is_some();
    use_future({
        let desktop = desktop.clone();
        move || {
            let desktop = desktop.clone();
            let tray = tray.clone();
            async move {
                let Some(tray) = tray.as_ref() else {
                    desktop.set_close_behavior(dioxus::desktop::WindowCloseBehaviour::WindowCloses);
                    return;
                };
                let _keep_alive = &tray.0;
                loop {
                    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                        if event.id == "open" {
                            desktop.window.set_visible(true);
                            desktop.window.set_focus();
                        } else if event.id == "exit" {
                            flush_web_progress().await;
                            desktop.set_close_behavior(
                                dioxus::desktop::WindowCloseBehaviour::WindowCloses,
                            );
                            shutdown_owned_processes();
                            desktop.close();
                            return;
                        }
                    }
                    while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                        if matches!(event, tray_icon::TrayIconEvent::DoubleClick { .. }) {
                            desktop.window.set_visible(true);
                            desktop.window.set_focus();
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    });
    let mut page = use_signal(|| Page::Home);
    let mut language = use_signal(|| {
        INITIAL_PREFERENCES
            .get()
            .map_or(Language::Russian, |preferences| preferences.language)
    });
    let mut query = use_signal(String::new);
    let mut endpoint = use_signal(|| {
        INITIAL_PREFERENCES
            .get()
            .map(|item| item.torrserver_url.clone())
            .unwrap_or_else(|| DEFAULT_TORRSERVER_URL.to_owned())
    });
    let mut server = use_signal(|| ServerState {
        version: String::new(),
        torrents: Vec::new(),
        error: String::new(),
    });
    let mut cards = use_signal(Vec::<LibraryCard>::new);
    let mut recent = use_signal(Vec::<ContinueItem>::new);
    let results = use_signal(Vec::<SearchResult>::new);
    let search_metadata = use_signal(|| None::<Movie>);
    let search_poster = use_signal(String::new);
    let mut selected = use_signal(|| None::<Torrent>);
    let mut files = use_signal(Vec::<PlayableFile>::new);
    let mut players = use_signal(Vec::<OwnedPlayer>::new);
    let mut web_player = use_signal(|| None::<WebPlayback>);
    let mut closing = use_signal(|| false);
    let close_window = desktop.clone();
    let _close_hook = dioxus::desktop::use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::{Event, WindowEvent};
        if let Event::WindowEvent {
            window_id,
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            if *window_id != close_window.window.id()
                || *closing.peek()
                || web_player.peek().is_none()
            {
                return;
            }
            let close_to_tray = settings_path()
                .ok()
                .and_then(|path| settings::load_preferences(&path).ok())
                .map(|preferences| preferences.close_to_tray)
                .unwrap_or(true);
            if close_to_tray && has_tray {
                return;
            }
            closing.set(true);
            close_window.set_close_behavior(dioxus::desktop::WindowCloseBehaviour::WindowHides);
            let close_window = close_window.clone();
            spawn(async move {
                flush_web_progress().await;
                close_window
                    .set_close_behavior(dioxus::desktop::WindowCloseBehaviour::WindowCloses);
                close_window.close();
            });
        }
    });
    let visibility_window = desktop.clone();
    use_future(move || {
        let visibility_window = visibility_window.clone();
        async move {
            loop {
                let visible = visibility_window.window.is_visible()
                    && !visibility_window.window.is_minimized();
                PLAYBACK_VISIBLE.store(visible, Ordering::Release);
                let mut eval = document::eval(&format!("if(window.__pirateCinemaVisible !== {visible}) {{ window.__pirateCinemaVisible={visible}; window.dispatchEvent(new Event('pc-visibility')); }} dioxus.send(true);"));
                let _ = eval.recv::<bool>().await;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    });
    let mut return_page = use_signal(|| Page::Library);
    let mut busy = use_signal(|| false);
    let mut metadata_status = use_signal(String::new);
    let mut pending_duplicate = use_signal(|| None::<(SearchResult, bool)>);
    let mut pending_playback = use_signal(|| None::<PendingPlayback>);
    let mut onboarding = use_signal(|| {
        INITIAL_PREFERENCES
            .get()
            .is_some_and(|preferences| preferences.onboarding_complete)
    });
    let mut startup_update = use_signal(|| None::<ReleaseUpdate>);

    use_effect(move || {
        let Some(playback) = web_player() else {
            return;
        };
        if *page.peek() != Page::Player {
            return_page.set(*page.peek());
            page.set(Page::Player);
        }
        let key = format!("{}-{}", playback.torrent.hash, playback.file.id);
        let selector = serde_json::to_string(&key).unwrap_or_else(|_| "\"\"".into());
        let episodes = playback
            .queue
            .iter()
            .map(|file| {
                serde_json::json!({
                    "id": file.id,
                    "name": file.name,
                    "current": file.id == playback.file.id,
                })
            })
            .collect::<Vec<_>>();
        let episodes = serde_json::to_string(&episodes).unwrap_or_else(|_| "[]".into());
        let next_episode_id = next_episode(&playback.queue, playback.file.id).map(|file| file.id);
        let next_episode_id =
            serde_json::to_string(&next_episode_id).unwrap_or_else(|_| "null".into());
        let torrent_hash =
            serde_json::to_string(&playback.torrent.hash).unwrap_or_else(|_| "\"\"".into());
        let current_file_id = playback.file.id;
        let audio_preference = history_path()
            .ok()
            .and_then(|path| HistoryStore::open(&path).ok())
            .and_then(|history| {
                history
                    .web_audio_preference(&playback.torrent.hash)
                    .ok()
                    .flatten()
            })
            .map(|(language, title)| serde_json::json!({"language":language,"title":title}));
        let audio_preference =
            serde_json::to_string(&audio_preference).unwrap_or_else(|_| "null".into());
        let auto_next = playback.auto_next;
        let script = format!(
            r#"
            {player_state_js}
            const key = {selector};
            const episodes = {episodes};
            const torrentHash = {torrent_hash};
            let currentFileId = {current_file_id};
            let nextEpisodeId = {next_episode_id};
            let container = null;
            for (let attempt = 0; attempt < 50; attempt++) {{
                container = document.querySelector(`[data-playback-key="${{key}}"]`);
                if (container) break;
                await new Promise(resolve => setTimeout(resolve, 100));
            }}
            if (!container) return;
            if (window.__pirateCinemaContainer === container && window.__pirateCinemaTorrent === torrentHash && window.__pirateCinemaSwitch) {{
                await window.__pirateCinemaSwitch(currentFileId, nextEpisodeId);
                return;
            }}
            {hls_js}
            {artplayer_js}
            if (!window.Artplayer || !window.Hls || !window.Hls.isSupported()) {{
                dioxus.send({{ kind: 'error', message: 'WebView2 не поддерживает ArtPlayer или Media Source Extensions' }});
                return;
            }}
            if (window.__pirateCinemaDispose) window.__pirateCinemaDispose();
            if (window.__pirateCinemaArt) window.__pirateCinemaArt.destroy();
            if (window.__pirateCinemaHls) window.__pirateCinemaHls.destroy();
            window.__pirateCinemaSwitch = null;
            window.__pirateCinemaTorrent = torrentHash;
            window.__pirateCinemaContainer = container;
            let switching = false;
            let sourceReady = false;
            let failSwitch = null;
            let audioPreference = {audio_preference};
            let autoNext = {auto_next};
            let selectedAudio = null;
            let audioTracks = [];
            let restoringAudio = false;
            let preparedFile = null;
            let pendingEnd = false;
            let retryPosition = 0;
            const canStart = () => window.PiratePlayerState.canStart(window.__pirateCinemaVisible, document.hidden);
            let hls = null;
            let video = null;
            let recoveryAttempts = 0;
            let playbackStarted = false;
            let streamHealthy = false;
            let restorePositionListener = null;
            const bufferedAhead = () => {{
                if (!video) return 0;
                const position = video.currentTime || 0;
                for (let index = 0; index < video.buffered.length; index++) {{
                    if (video.buffered.start(index) <= position + 0.25 && video.buffered.end(index) >= position) {{
                        return video.buffered.end(index) - position;
                    }}
                }}
                return 0;
            }};
            const startWhenBuffered = () => {{
                if (playbackStarted || !canStart() || !window.PiratePlayerState.startupReady(video.duration, video.currentTime || 0, bufferedAhead())) return;
                playbackStarted = true;
                video.play().catch(() => {{ playbackStarted = false; }});
            }};
            const attachHls = (target, url) => {{
                if (hls) hls.destroy();
                if (video && restorePositionListener) video.removeEventListener('loadedmetadata',restorePositionListener);
                video = target;
                sourceReady = false;
                recoveryAttempts = 0;
                playbackStarted = false;
                streamHealthy = false;
                const source = new URL(url);
                const startPosition = Number(source.searchParams.get('seconds')) || 0;
                source.searchParams.set('seconds', '0');
                hls = new window.Hls({{
                    startPosition,
                    enableWorker: true,
                    startFragPrefetch: true,
                    maxBufferLength: 60,
                    maxMaxBufferLength: 120,
                    backBufferLength: 30,
                    maxBufferHole: 0.5
                }});
                window.__pirateCinemaHls = hls;
                const restorePosition = () => {{
                    if (startPosition > 0 && Math.abs((video.currentTime || 0) - startPosition) > 1) {{
                        video.currentTime = startPosition;
                    }}
                }};
                restorePositionListener=restorePosition;
                target.addEventListener('loadedmetadata', restorePosition, {{ once: true }});
                hls.on(window.Hls.Events.FRAG_BUFFERED, startWhenBuffered);
                hls.on(window.Hls.Events.FRAG_LOADED, () => {{
                    recoveryAttempts = 0;
                    if (!streamHealthy) {{
                        streamHealthy = true;
                        dioxus.send({{ kind: 'healthy' }});
                    }}
                }});
                hls.on(window.Hls.Events.ERROR, (_event, data) => {{
                    if (!data.fatal) return;
                    if (recoveryAttempts++ < 3 && data.type === window.Hls.ErrorTypes.NETWORK_ERROR) {{
                        hls.startLoad(video.currentTime || -1);
                        return;
                    }}
                    if (recoveryAttempts <= 3 && data.type === window.Hls.ErrorTypes.MEDIA_ERROR) {{
                        hls.recoverMediaError();
                        return;
                    }}
                    dioxus.send({{ kind: 'error', message: data.details || data.type || 'Ошибка HLS' }});
                    showStatus('Не удалось загрузить поток', true);
                    if (failSwitch) failSwitch(new Error(data.details || 'Ошибка HLS'));
                }});
                hls.subtitleDisplay = true;
                hls.on(window.Hls.Events.SUBTITLE_TRACKS_UPDATED, (_event, data) => {{
                    subtitleTracks = data.subtitleTracks || [];
                    updateSubtitles();
                }});
                hls.loadSource(source.toString());
                hls.attachMedia(target);
            }};
            const art = new window.Artplayer({{
                container,
                url: container.dataset.hlsUrl,
                type: 'm3u8',
                title: container.dataset.title,
                theme: '#287cff',
                lang: 'ru',
                i18n: {{
                    ru: {{
                        'Play': 'Воспроизвести', 'Pause': 'Пауза', 'Replay': 'Повторить',
                        'Volume': 'Громкость', 'Mute': 'Без звука', 'Video Info': 'О видео', 'Close': 'Закрыть',
                        'Rate': 'Скорость', 'Default': 'По умолчанию', 'Normal': 'Обычная', 'Open': 'Открыть',
                        'Play Speed': 'Скорость', 'Aspect Ratio': 'Соотношение сторон',
                        'Fullscreen': 'Полный экран', 'Exit Fullscreen': 'Выйти из полного экрана',
                        'Web Fullscreen': 'На всё окно', 'Exit Web Fullscreen': 'Выйти из режима окна',
                        'PIP Mode': 'Картинка в картинке', 'Exit PIP Mode': 'Выйти из PiP',
                        'Mini Player': 'Мини-плеер', 'Video Flip': 'Отразить видео',
                        'Horizontal': 'Горизонтально', 'Vertical': 'Вертикально', 'Reconnect': 'Переподключиться',
                        'Show Setting': 'Показать настройки', 'Hide Setting': 'Скрыть настройки', 'Screenshot': 'Снимок кадра',
                        'Switch Video': 'Сменить видео', 'Switch Subtitle': 'Сменить субтитры',
                        'Subtitle Offset': 'Смещение субтитров', 'Video Load Failed': 'Не удалось загрузить видео',
                        'PIP Not Supported': 'Режим PiP не поддерживается', 'Fullscreen Not Supported': 'Полный экран не поддерживается',
                        'Last Seen': 'Последняя позиция', 'Jump Play': 'Перейти к просмотру',
                        'AirPlay': 'AirPlay', 'AirPlay Not Available': 'AirPlay недоступен'
                    }}
                }},
                autoplay: false,
                fullscreen: true,
                fullscreenWeb: true,
                pip: true,
                hotkey: true,
                setting: true,
                playbackRate: true,
                aspectRatio: true,
                customType: {{ m3u8: attachHls }}
            }});
            window.__pirateCinemaArt = art;
            // ArtPlayer's customType callback runs asynchronously after construction.
            video = art.template.$video;
            video.addEventListener('loadedmetadata', () => {{sourceReady=true;}});
            let subtitleTracks = [];
            let subtitleBlob = null;
            let localSubtitleTrack = null;
            const localSubtitle = document.createElement('input');
            localSubtitle.type='file'; localSubtitle.accept='.srt,.vtt';
            const clearLocalSubtitle = () => {{
                art.subtitle.show=false;
                if(localSubtitleTrack) localSubtitleTrack.remove();
                localSubtitleTrack=null;
                if (subtitleBlob) URL.revokeObjectURL(subtitleBlob);
                subtitleBlob=null;
            }};
            localSubtitle.addEventListener('change', async () => {{
                const file=localSubtitle.files?.[0];
                if (!file) return;
                if (file.size > 2 * 1024 * 1024 || !/\.(srt|vtt)$/i.test(file.name)) {{ art.notice.show='Нужен SRT/VTT размером до 2 МБ'; return; }}
                const fileId=currentFileId;
                const text=await file.text();
                if (fileId !== currentFileId) return;
                clearLocalSubtitle();
                if(hls) hls.subtitleTrack=-1;
                subtitleBlob=URL.createObjectURL(new Blob([window.PiratePlayerState.subtitleVtt(text)],{{type:'text/vtt'}}));
                // Native track avoids ArtPlayer's textTracks[0] assumption with HLS subtitles.
                localSubtitleTrack=document.createElement('track');
                localSubtitleTrack.kind='subtitles';
                localSubtitleTrack.label=file.name;
                localSubtitleTrack.src=subtitleBlob;
                video.append(localSubtitleTrack);
                localSubtitleTrack.track.mode='showing';
                art.notice.show=file.name;
                localSubtitle.value='';
            }});
            const subtitleSetting = {{
                name:'pc-subtitles',html:'Субтитры',selector:[{{html:'Отключены',value:-1}},{{html:'Локальный SRT/VTT…',value:-2}}],
                onSelect: item => {{
                    if(item.value===-2) {{localSubtitle.click();return 'Локальный файл';}}
                    clearLocalSubtitle();
                    if(hls) {{hls.subtitleDisplay=true;hls.subtitleTrack=item.value;}}
                    return item.html;
                }}
            }};
            art.setting.add(subtitleSetting);
            const updateSubtitles = () => art.setting.update({{...subtitleSetting,selector:[
                {{html:'Отключены',value:-1}},
                ...subtitleTracks.map((track,index) => ({{html:String(track.name || track.lang || `Дорожка ${{index+1}}`).replace(/[<>&"']/g,''),value:index}})),
                {{html:'Локальный SRT/VTT…',value:-2}}
            ]}});
            const saveProgress = () => {{
                if (!switching && sourceReady && art.duration > 0) dioxus.send({{ kind:'progress', file_id:currentFileId, position:art.currentTime || 0, duration:art.duration || 0 }});
            }};
            window.__pirateCinemaSave = saveProgress;
            window.__pirateCinemaSnapshot = () => switching || !sourceReady || art.duration <= 0 ? null : ({{hash:torrentHash,file_id:currentFileId,position:art.currentTime || 0,duration:art.duration || 0}});
            const status = document.createElement('div');
            status.className = 'pc-stream-status';
            status.setAttribute('role', 'status');
            const statusText = document.createElement('span');
            const retryButton = document.createElement('button');
            retryButton.textContent = 'Повторить';
            retryButton.type = 'button';
            retryButton.hidden = true;
            status.append(statusText, retryButton);
            art.template.$player.append(status);
            const showStatus = (text, retry = false) => {{ statusText.textContent=text; retryButton.hidden=!retry; status.hidden=!text; }};
            showStatus(canStart() ? 'Подготовка потока…' : 'Ожидание восстановления окна…');
            art.on('video:playing', () => showStatus(''));
            art.on('video:waiting', () => {{ retryPosition = art.currentTime || retryPosition; showStatus('Буферизация…'); }});
            art.on('video:pause', saveProgress);
            art.on('video:seeked', saveProgress);
            art.on('video:play', () => {{ if (!canStart()) {{ art.pause(); showStatus('Ожидание восстановления окна…'); }} }});
            const onVisibility = () => {{
                saveProgress();
                if (canStart()) {{
                    if (pendingEnd && autoNext) {{ pendingEnd=false; dioxus.send({{kind:'ended',source_file_id:currentFileId,auto_next:autoNext}}); }}
                    else startWhenBuffered();
                }}
            }};
            document.addEventListener('visibilitychange', onVisibility);
            window.addEventListener('pc-visibility', onVisibility);
            const retryStream = () => {{
                if (switching) return;
                saveProgress();
                retryPosition = art.currentTime || retryPosition;
                const url = new URL(art.url || container.dataset.hlsUrl);
                url.searchParams.set('seconds', String(Math.floor(retryPosition)));
                art.pause();
                attachHls(art.template.$video, url.toString());
                showStatus('Переподключение…');
            }};
            retryButton.addEventListener('click', retryStream);
            const switchSource = url => new Promise((resolve,reject) => {{
                switching=true;
                art.pause();
                const ready=() => finish();
                const finish=error => {{
                    clearTimeout(timer);
                    video.removeEventListener('loadedmetadata',ready);
                    failSwitch=null;
                    switching=false;
                    if(error) reject(error); else resolve();
                }};
                const timer=setTimeout(() => finish(new Error('Поток не подготовлен за 40 секунд')),40000);
                failSwitch=finish;
                video.addEventListener('loadedmetadata',ready,{{once:true}});
                try {{ art.url=url; }} catch(error) {{ finish(error); }}
            }});
            const topbar = document.createElement('div');
            topbar.className = 'pc-player-topbar';
            const addSelect = (className, label) => {{
                const select = document.createElement('select');
                select.className = className;
                select.title = label;
                select.setAttribute('aria-label', label);
                topbar.append(select);
                return select;
            }};
            const parsedEpisodes = episodes.map((item, index) => {{
                const match = item.name.match(/\bS(\d{{1,2}})E(\d{{1,3}})\b/i);
                return {{ ...item, season: match ? Number(match[1]) : 1, episode: match ? Number(match[2]) : index + 1 }};
            }});
            let seasonSelect = null;
            let episodeSelect = null;
            let selectedSeason = parsedEpisodes.find(item => item.id === currentFileId)?.season ?? 1;
            let fillEpisodes = () => {{}};
            if (episodes.length > 1) {{
                const seasons = [...new Set(parsedEpisodes.map(item => item.season))];
                selectedSeason = parsedEpisodes.find(item => item.id === currentFileId)?.season ?? seasons[0];
                episodeSelect = addSelect('pc-episode', 'Выбор серии');
                fillEpisodes = () => {{
                    const choices = parsedEpisodes.filter(item => item.season === selectedSeason);
                    episodeSelect.replaceChildren(...choices.map(item => new Option(`Серия ${{item.episode}}`, String(item.id), false, item.id === currentFileId)));
                    if (!choices.some(item => item.id === currentFileId)) episodeSelect.value = String(choices[0]?.id ?? '');
                }};
                if (seasons.length > 1) {{
                    seasonSelect = addSelect('pc-season', 'Выбор сезона');
                    seasonSelect.replaceChildren(...seasons.map(season => new Option(`Сезон ${{season}}`, String(season), false, season === selectedSeason)));
                    topbar.insertBefore(seasonSelect, episodeSelect);
                    seasonSelect.addEventListener('change', () => {{ selectedSeason = Number(seasonSelect.value); fillEpisodes(); }});
                }}
                fillEpisodes();
                episodeSelect.addEventListener('change', () => {{
                    if (switching) {{ fillEpisodes(); return; }}
                    const fileId = Number(episodeSelect.value);
                    if (fileId !== currentFileId && canStart()) {{ saveProgress(); dioxus.send({{ kind: 'episode', file_id: fileId, source_file_id: currentFileId,auto_next:autoNext }}); }}
                }});
            }}
            const audioSelect = addSelect('pc-audio', 'Выбор озвучки');
            audioSelect.append(new Option('Озвучка…', ''));
            art.template.$player.append(topbar);
            const autoLabel = document.createElement('label');
            autoLabel.className='pc-auto-next';
            const autoCheck=document.createElement('input');
            autoCheck.type='checkbox'; autoCheck.checked=autoNext;
            autoLabel.append(autoCheck, ' Автопереход');
            if (episodes.length > 1) topbar.append(autoLabel);
            autoCheck.addEventListener('change', () => {{ autoNext=autoCheck.checked; dioxus.send({{kind:'auto_next',active:autoNext}}); }});
            const nextButton = document.createElement('button');
            nextButton.className = 'pc-next-episode';
            nextButton.type = 'button';
            nextButton.textContent = 'Следующая серия';
            nextButton.setAttribute('aria-label', 'Следующая серия');
            art.template.$player.append(nextButton);
            const updateNextButton = () => {{
                const duration = art.duration;
                const remaining = duration - art.currentTime;
                nextButton.style.display = !switching && nextEpisodeId !== null && Number.isFinite(duration) && duration > 0 && remaining > 0 && remaining <= 120 ? 'block' : 'none';
                if (nextEpisodeId !== null && window.PiratePlayerState.shouldPrepare(remaining, bufferedAhead(), canStart(), preparedFile === nextEpisodeId)) {{
                    preparedFile=nextEpisodeId;
                    const probe = new URL(container.dataset.probeUrl);
                    probe.searchParams.set('index', String(nextEpisodeId));
                    fetch(probe, {{signal:AbortSignal.timeout(4000)}}).catch(() => {{}});
                }}
            }};
            art.template.$video.addEventListener('timeupdate', updateNextButton);
            art.template.$video.addEventListener('durationchange', updateNextButton);
            nextButton.addEventListener('click', () => {{
                if (switching || !canStart() || nextEpisodeId === null) return;
                saveProgress();
                dioxus.send({{ kind: 'episode', file_id: nextEpisodeId, source_file_id: currentFileId, resume: false,auto_next:autoNext }});
            }});
            requestAnimationFrame(() => {{
                if (!canStart()) return;
                window.focus();
                container.scrollIntoView({{ block: 'start', behavior: 'smooth' }});
                container.tabIndex = -1;
                container.focus({{ preventScroll: true }});
            }});
            art.on('fullscreen', active => dioxus.send({{ kind: 'fullscreen', active }}));
            art.on('video:ended', () => {{
                saveProgress();
                if (!switching && autoNext) {{
                    if (canStart()) dioxus.send({{ kind: 'ended', source_file_id: currentFileId,auto_next:autoNext }});
                    else pendingEnd=true;
                }}
            }});
            audioSelect.addEventListener('change', () => {{
                if (switching) return;
                saveProgress();
                const track = audioTracks.find(track => String(track.Index) === audioSelect.value);
                if (track && !restoringAudio) {{
                    audioPreference={{language:track.Language || '',title:track.Title || ''}};
                    dioxus.send({{kind:'audio_preference',...audioPreference}});
                }}
                selectedAudio=audioSelect.value;
                restoringAudio=false;
                const next = new URL(container.dataset.hlsUrl);
                next.searchParams.set('audio', audioSelect.value);
                next.searchParams.set('seconds', String(Math.floor(art.currentTime || 0)));
                switchSource(next.toString())
                    .catch(error => {{ if(window.__pirateCinemaArt !== art) return; showStatus(String(error),true); dioxus.send({{ kind: 'error', message: String(error) }}); }});
            }});
            const loadAudioTracks = async () => {{
                const probeUrl = container.dataset.probeUrl;
                for (let attempt = 0; attempt < 4; attempt++) {{
                    try {{
                        const response = await fetch(probeUrl, {{signal:AbortSignal.timeout(5000)}});
                        const info = response.ok ? await response.json() : null;
                        if (window.__pirateCinemaArt !== art || probeUrl !== container.dataset.probeUrl) return;
                        const tracks = (info?.Tracks || []).filter(track => String(track.Type).toLowerCase() === 'audio');
                        if (!tracks.length) throw new Error('audio tracks unavailable');
                        audioTracks=tracks.map((track,index) => ({{...track,Index:track.Index ?? index}}));
                        audioSelect.replaceChildren(...tracks.map((track, index) => new Option(
                            [track.Title, track.Language, track.Channels ? `${{track.Channels}} ch` : ''].filter(Boolean).join(' · ') || `Дорожка ${{index + 1}}`,
                            String(track.Index ?? index)
                        )));
                        const preferred=window.PiratePlayerState.preferredTrack(audioTracks,audioPreference);
                        if (preferred >= 0) {{
                            audioSelect.value=String(audioTracks[preferred].Index);
                            if (selectedAudio !== audioSelect.value && !switching) {{ restoringAudio=true; audioSelect.dispatchEvent(new Event('change')); }}
                        }}
                        return;
                    }} catch (_) {{
                        if (attempt < 3) await new Promise(resolve => setTimeout(resolve, 2000));
                    }}
                }}
                if (window.__pirateCinemaArt !== art || probeUrl !== container.dataset.probeUrl) return;
                audioSelect.replaceChildren(new Option('Озвучка недоступна', ''));
                audioSelect.disabled = true;
            }};
            window.__pirateCinemaSwitch = async (fileId, followingId) => {{
                switching = true;
                pendingEnd=false;
                clearLocalSubtitle();
                subtitleTracks=[];
                updateSubtitles();
                selectedAudio=null;
                retryPosition=0;
                preparedFile=null;
                currentFileId = fileId;
                nextEpisodeId = followingId;
                nextButton.style.display = 'none';
                const current = parsedEpisodes.find(item => item.id === fileId);
                if (current && episodeSelect) {{
                    selectedSeason = current.season;
                    if (seasonSelect) seasonSelect.value = String(selectedSeason);
                    fillEpisodes();
                }}
                audioSelect.disabled = false;
                audioSelect.replaceChildren(new Option('Озвучка…', ''));
                try {{
                    await switchSource(container.dataset.hlsUrl);
                    void loadAudioTracks();
                }} catch (error) {{
                    if(window.__pirateCinemaArt !== art) return;
                    showStatus(String(error),true);
                    dioxus.send({{ kind: 'error', message: String(error) }});
                }} finally {{
                    switching = false;
                }}
            }};
            void loadAudioTracks();
            const dispose = () => {{
                document.removeEventListener('visibilitychange',onVisibility);
                window.removeEventListener('pc-visibility',onVisibility);
                clearLocalSubtitle();
                if(failSwitch) failSwitch(new Error('Плеер закрыт'));
            }};
            window.__pirateCinemaDispose=dispose;
            while (document.contains(container) && window.__pirateCinemaArt === art) {{
                await new Promise(resolve => setTimeout(resolve, 5000));
                fetch(container.dataset.heartbeatUrl, {{signal:AbortSignal.timeout(4000)}}).then(async response => {{
                    if (!response.ok || status.hidden || !retryButton.hidden) return;
                    const state=await response.json();
                    if ((state.active_peers ?? state.ActivePeers) === 0) showStatus('Поиск пиров…');
                }}).catch(() => {{}});
                saveProgress();
            }}
            dispose();
            if (window.__pirateCinemaArt === art) {{
                window.__pirateCinemaArt = null;
                window.__pirateCinemaSwitch = null;
                window.__pirateCinemaTorrent = null;
                window.__pirateCinemaContainer = null;
                window.__pirateCinemaSave = null;
                window.__pirateCinemaSnapshot = null;
                window.__pirateCinemaDispose = null;
                if (window.__pirateCinemaHls === hls) window.__pirateCinemaHls = null;
                if (hls) hls.destroy();
                art.destroy();
            }}
            "#,
            hls_js = HLS_JS,
            artplayer_js = ARTPLAYER_JS,
            episodes = episodes,
            next_episode_id = next_episode_id,
            torrent_hash = torrent_hash,
            current_file_id = current_file_id,
            player_state_js = PLAYER_STATE_JS,
            audio_preference = audio_preference,
            auto_next = auto_next
        );
        let player_window = desktop.clone();
        spawn(async move {
            let mut eval = document::eval(&script);
            while let Ok(value) = eval.recv::<serde_json::Value>().await {
                let Some(current_playback) = web_player.read().clone() else {
                    break;
                };
                if current_playback.torrent.hash != playback.torrent.hash {
                    break;
                }
                let playback = current_playback;
                if value.get("kind").and_then(|item| item.as_str()) == Some("audio_preference") {
                    if let Ok(path) = history_path() {
                        if let Ok(history) = HistoryStore::open(&path) {
                            let _ = history.save_web_audio_preference(
                                &playback.torrent.hash,
                                value
                                    .get("language")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or(""),
                                value
                                    .get("title")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or(""),
                            );
                        }
                    }
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("auto_next") {
                    if let Ok(path) = history_path() {
                        if let Ok(history) = HistoryStore::open(&path) {
                            if let Ok(mut preferences) =
                                history.playback_preferences(&playback.torrent.hash)
                            {
                                preferences.auto_next = value
                                    .get("active")
                                    .and_then(|item| item.as_bool())
                                    .unwrap_or(false);
                                let _ = history.save_playback_preferences(
                                    &playback.torrent.hash,
                                    &preferences,
                                );
                            }
                        }
                    }
                    continue;
                }
                if matches!(
                    value.get("kind").and_then(|item| item.as_str()),
                    Some("episode" | "ended")
                ) && (busy()
                    || value.get("source_file_id").and_then(|item| item.as_i64())
                        != Some(playback.file.id))
                {
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("fullscreen") {
                    player_window.set_fullscreen(
                        value
                            .get("active")
                            .and_then(|item| item.as_bool())
                            .unwrap_or(false),
                    );
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("error") {
                    let message = value
                        .get("message")
                        .and_then(|item| item.as_str())
                        .unwrap_or("Неизвестная ошибка HLS");
                    server.write().error = format!("Встроенный HLS-плеер: {message}");
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("healthy") {
                    if server.read().error.starts_with("Встроенный HLS-плеер:") {
                        server.write().error.clear();
                    }
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("episode") {
                    if let Some(file) = value
                        .get("file_id")
                        .and_then(|item| item.as_i64())
                        .and_then(|id| playback.queue.iter().find(|file| file.id == id))
                        .cloned()
                    {
                        launch_playback(
                            PendingPlayback {
                                torrent: playback.torrent.clone(),
                                file,
                                resume: value
                                    .get("resume")
                                    .and_then(|item| item.as_bool())
                                    .unwrap_or(true),
                                queue: playback.queue.clone(),
                                auto_next: value
                                    .get("auto_next")
                                    .and_then(|item| item.as_bool())
                                    .unwrap_or(playback.auto_next),
                                force_mpv: false,
                            },
                            endpoint(),
                            busy,
                            players,
                            web_player,
                            server,
                        );
                    }
                    continue;
                }
                if value.get("kind").and_then(|item| item.as_str()) == Some("ended") {
                    if value
                        .get("auto_next")
                        .and_then(|item| item.as_bool())
                        .unwrap_or(playback.auto_next)
                    {
                        if let Some(file) = playback
                            .queue
                            .iter()
                            .position(|file| file.id == playback.file.id)
                            .and_then(|position| playback.queue.get(position + 1))
                            .cloned()
                        {
                            launch_playback(
                                PendingPlayback {
                                    torrent: playback.torrent.clone(),
                                    file,
                                    resume: true,
                                    queue: playback.queue.clone(),
                                    auto_next: true,
                                    force_mpv: false,
                                },
                                endpoint(),
                                busy,
                                players,
                                web_player,
                                server,
                            );
                        }
                    }
                    continue;
                }
                if value.get("file_id").and_then(|item| item.as_i64()) != Some(playback.file.id) {
                    continue;
                }
                let position = value
                    .get("position")
                    .and_then(|item| item.as_f64())
                    .unwrap_or(0.0);
                let duration = value
                    .get("duration")
                    .and_then(|item| item.as_f64())
                    .unwrap_or(0.0);
                if let Ok(path) = history_path() {
                    if let Ok(history) = HistoryStore::open(&path) {
                        let _ = history.save_progress(
                            &playback.torrent.hash,
                            playback.file.id,
                            position as i64,
                            duration as i64,
                        );
                    }
                }
            }
        });
    });

    use_future(move || async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if let Ok(Ok(update)) = tokio::task::spawn_blocking(|| check_rust_update(VERSION)).await {
            startup_update.set(update);
        }
    });

    use_future(move || async move {
        for _ in 0..20 {
            if !server.read().version.is_empty() {
                break;
            }
            refresh_server(endpoint(), busy, server, cards, recent);
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    use_future(move || async move {
        for _ in 0..20 {
            let missing = missing_metadata_torrents(&cards());
            if !missing.is_empty() {
                let request_endpoint = endpoint();
                let _ = tokio::task::spawn_blocking(move || {
                    sync_library_metadata(&request_endpoint, &missing, None)
                })
                .await;
                cards.set(load_library_cards(&server.read().torrents).unwrap_or_default());
                return;
            }
            if !server.read().version.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    use_future(move || async move {
        if let Some(magnet) = STARTUP_MAGNET.get().cloned() {
            let title = pirate_cinema_core::magnet_title(&magnet);
            busy.set(true);
            let endpoint_value = endpoint();
            spawn(async move {
                let request_endpoint = endpoint_value.clone();
                let request_title = title.clone();
                let added = tokio::task::spawn_blocking(move || {
                    add_magnet(&request_endpoint, &magnet, &request_title)
                })
                .await;
                match added {
                    Ok(Ok(added)) => open_torrent(
                        Torrent {
                            title,
                            hash: added.hash,
                        },
                        endpoint_value,
                        page,
                        busy,
                        selected,
                        files,
                        server,
                    ),
                    Ok(Err(error)) => {
                        server.write().error = error;
                        busy.set(false);
                    }
                    Err(error) => {
                        server.write().error = error.to_string();
                        busy.set(false);
                    }
                }
            });
        }
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(750)).await;
            let mut remove = Vec::new();
            let mut advance = Vec::new();
            let mut refresh_history = false;
            let mut playback_error = None;
            {
                let active = players.read();
                for (index, player) in active.iter().enumerate() {
                    loop {
                        match player.session.try_recv() {
                            Ok(MpvEvent::Loaded | MpvEvent::Progress { .. }) => {
                                refresh_history = true
                            }
                            Ok(MpvEvent::Ended { eof }) => {
                                refresh_history = true;
                                if eof && player.auto_next {
                                    if let Some(position) = player
                                        .queue
                                        .iter()
                                        .position(|file| file.id == player.file.id)
                                    {
                                        if let Some(file) = player.queue.get(position + 1).cloned()
                                        {
                                            advance.push(PendingPlayback {
                                                torrent: player.torrent.clone(),
                                                file,
                                                resume: true,
                                                queue: player.queue.clone(),
                                                auto_next: true,
                                                force_mpv: false,
                                            });
                                        }
                                    }
                                }
                                remove.push(index);
                                break;
                            }
                            Ok(MpvEvent::StreamFailed(error) | MpvEvent::Error(error)) => {
                                playback_error = Some(error)
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                remove.push(index);
                                break;
                            }
                        }
                    }
                }
            }
            if !remove.is_empty() {
                remove.sort_unstable();
                remove.dedup();
                let mut active = players.write();
                for index in remove.into_iter().rev() {
                    if index < active.len() {
                        active.remove(index);
                    }
                }
            }
            if refresh_history {
                let torrents = server.read().torrents.clone();
                cards.set(load_library_cards(&torrents).unwrap_or_default());
                recent.set(load_continue_items(&torrents).unwrap_or_default());
            }
            if let Some(error) = playback_error {
                server.write().error = error;
            }
            for request in advance {
                launch_playback(request, endpoint(), busy, players, web_player, server);
            }
        }
    });

    let open_saved = move |torrent: Torrent| {
        open_torrent(torrent, endpoint(), page, busy, selected, files, server);
    };
    let mut add_result = move |(item, open): (SearchResult, bool)| {
        let existing = {
            let state = server.read();
            state
                .torrents
                .iter()
                .find(|torrent| torrent.hash.eq_ignore_ascii_case(&item.hash))
                .cloned()
        };
        if let Some(existing) = existing {
            if open {
                open_torrent(existing, endpoint(), page, busy, selected, files, server);
            } else {
                server.write().error = "Эта раздача уже сохранена в TorrServer".into();
            }
        } else if server.read().torrents.iter().any(|torrent| {
            !torrent.hash.eq_ignore_ascii_case(&item.hash)
                && metadata::same_release(&torrent.title, &item.title)
        }) {
            pending_duplicate.set(Some((item, open)));
        } else {
            add_search_result(
                item,
                open,
                endpoint(),
                page,
                busy,
                selected,
                files,
                server,
                cards,
                recent,
            );
        }
    };

    let mut run_search = move || {
        let value = query.read().trim().to_owned();
        if value.len() < 2 {
            return;
        }
        if value.to_ascii_lowercase().starts_with("magnet:?") {
            let Some(hash) = magnet_info_hash(&value) else {
                server.write().error = "Magnet-ссылка не содержит корректный BTIH".into();
                return;
            };
            add_result((
                SearchResult {
                    title: magnet_title(&value),
                    magnet: value,
                    hash,
                    size: String::new(),
                    seeders: 0,
                    source: "Magnet".into(),
                    tracker: String::new(),
                },
                true,
            ));
        } else {
            start_search(
                value,
                endpoint(),
                page,
                busy,
                results,
                search_metadata,
                search_poster,
                server,
            );
        }
    };

    let play_file = move |(file, resume, auto_next): (VideoFile, bool, bool)| {
        let Some(torrent) = selected.read().clone() else {
            return;
        };
        if web_player.read().as_ref().is_some_and(|playback| {
            playback.torrent.hash == torrent.hash && playback.file.id == file.id
        }) {
            return_page.set(page());
            page.set(Page::Player);
            return;
        }
        if let Some(session) = players
            .write()
            .iter_mut()
            .find(|player| player.session.matches(&torrent.hash, file.id))
        {
            server.write().error = match session.session.focus() {
                Ok(()) => "Окно MPV уже открыто".into(),
                Err(error) => error,
            };
            return;
        }
        let queue = files
            .read()
            .iter()
            .map(|item| item.file.clone())
            .collect::<Vec<_>>();
        if players.read().is_empty() && web_player.read().is_none() {
            launch_playback(
                PendingPlayback {
                    torrent,
                    file,
                    resume,
                    queue,
                    auto_next,
                    force_mpv: false,
                },
                endpoint(),
                busy,
                players,
                web_player,
                server,
            );
        } else {
            pending_playback.set(Some(PendingPlayback {
                torrent,
                file,
                resume,
                queue,
                auto_next,
                force_mpv: false,
            }));
        }
    };

    let update_media_type = move |kind: String| {
        let Some(torrent) = selected.read().clone() else {
            return;
        };
        let result = history_path()
            .and_then(|path| HistoryStore::open(&path).map_err(|error| error.to_string()))
            .and_then(|history| {
                history
                    .set_media_type(&torrent.hash, &kind)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => cards.set(load_library_cards(&server.read().torrents).unwrap_or_default()),
            Err(error) => server.write().error = error,
        }
    };

    let update_title = move |title: String| {
        let Some(torrent) = selected.read().clone() else {
            return;
        };
        let result = history_path()
            .and_then(|path| HistoryStore::open(&path).map_err(|error| error.to_string()))
            .and_then(|history| {
                history
                    .set_title(&torrent.hash, &title)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => cards.set(load_library_cards(&server.read().torrents).unwrap_or_default()),
            Err(error) => server.write().error = error,
        }
    };

    let refresh_selected_metadata = move |_| {
        let Some(torrent) = selected.read().clone() else {
            return;
        };
        busy.set(true);
        metadata_status.set("Обновляем описание и постер…".into());
        let endpoint_value = endpoint();
        spawn(async move {
            let torrent_for_request = torrent.clone();
            let result = tokio::task::spawn_blocking(move || {
                sync_library_metadata(&endpoint_value, &[torrent_for_request], None)
            })
            .await;
            match result {
                Ok(Ok((updated, failed, _))) => {
                    cards.set(load_library_cards(&server.read().torrents).unwrap_or_default());
                    metadata_status.set(format!(
                        "Карточка обновлена: {updated}, без результата: {failed}"
                    ));
                }
                Ok(Err(error)) => metadata_status.set(error),
                Err(error) => metadata_status.set(error.to_string()),
            }
            busy.set(false);
        });
    };

    let remove_saved = move |_| {
        let Some(torrent) = selected.read().clone() else {
            return;
        };
        let endpoint_value = endpoint();
        busy.set(true);
        spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || remove_torrent(&endpoint_value, &torrent.hash))
                    .await;
            match result {
                Ok(Ok(())) => {
                    selected.set(None);
                    files.set(Vec::new());
                    page.set(Page::Library);
                    refresh_server(endpoint(), busy, server, cards, recent);
                }
                Ok(Err(error)) => {
                    server.write().error = error;
                    busy.set(false);
                }
                Err(error) => {
                    server.write().error = error.to_string();
                    busy.set(false);
                }
            }
        });
    };

    let sync_metadata = move |_| {
        if busy() {
            return;
        }
        let torrents = server.read().torrents.clone();
        let endpoint_value = endpoint();
        busy.set(true);
        metadata_status.set(format!("Обновляем карточки: 0/{}", torrents.len()));
        spawn(async move {
            let total = torrents.len();
            let progress = Arc::new(AtomicUsize::new(0));
            let worker_progress = progress.clone();
            let mut task = tokio::task::spawn_blocking(move || {
                sync_library_metadata(&endpoint_value, &torrents, Some(&worker_progress))
            });
            let result = loop {
                tokio::select! {
                    result = &mut task => break result,
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {
                        metadata_status.set(format!("Обновляем карточки: {}/{total}", progress.load(Ordering::Relaxed)));
                    }
                }
            };
            match result {
                Ok(Ok((updated, failed, refreshed))) => {
                    cards.set(refreshed);
                    metadata_status.set(format!(
                        "Карточки обновлены: {updated}, без результата: {failed}"
                    ));
                }
                Ok(Err(error)) => metadata_status.set(error),
                Err(error) => metadata_status.set(error.to_string()),
            }
            busy.set(false);
        });
    };

    let player_labels = players
        .read()
        .iter()
        .map(|player| player.session.label().to_owned())
        .collect::<Vec<_>>();
    rsx! {
        style { {CSS} }
        document::Title { "Pirate Cinema {VERSION} — локальная медиатека" }
        if !onboarding() {
            Welcome { on_complete: move |preferences: Preferences| {
                language.set(preferences.language);
                endpoint.set(preferences.torrserver_url.clone());
                let result = settings_path().and_then(|path| settings::save_preferences(&path, &preferences));
                match result {
                    Ok(()) => { onboarding.set(true); refresh_server(endpoint(), busy, server, cards, recent); }
                    Err(error) => server.write().error = error,
                }
            } }
        } else { div { class: "shell",
            aside { class: "sidebar",
                div { class: "brand", img { src: "{brand_icon}", alt: "" } span { "Pirate Cinema" } }
                nav { class: "nav",
                    NavButton { label: language().pick("Главная", "Home"), icon: home_icon.clone(), active: page() == Page::Home, onclick: move |_| page.set(Page::Home) }
                    NavButton { label: language().pick("Поиск", "Search"), icon: String::new(), active: page() == Page::Search, onclick: move |_| page.set(Page::Search) }
                    NavButton { label: language().pick("Медиатека", "Library"), icon: library_icon.clone(), active: page() == Page::Library, onclick: move |_| page.set(Page::Library) }
                }
                div { class: "rail-bottom",
                    NavButton { label: language().pick("Настройки", "Settings"), icon: settings_icon.clone(), active: page() == Page::Settings, onclick: move |_| page.set(Page::Settings) }
                }
                div { class: "version", "Pirate Cinema · {VERSION}" }
            }
            main { class: "content",
                header { class: "topbar",
                    form { class: "search", onsubmit: move |event| { event.prevent_default(); run_search(); },
                        i { class: "search-icon" }
                        input {
                            value: "{query}",
                            placeholder: language().pick("Найти фильм, сериал или вставить magnet-ссылку", "Find a movie, series, or paste a magnet link"),
                            oninput: move |event| query.set(event.value())
                        }
                        button { r#type: "submit", disabled: busy(), if busy() { "…" } else { {language().pick("Найти", "Find")} } }
                    }
                    div { class: if server.read().version.is_empty() { "status offline" } else { "status" },
                        img { src: "{online_icon}", alt: "" }
                        if server.read().version.is_empty() { {language().pick("TorrServer офлайн", "TorrServer offline")} } else { "TorrServer · {server.read().version}" }
                    }
                }
                if !server.read().error.is_empty() { div { class: "notice", "{server.read().error}" } }
                if let Some(error) = STARTUP_MIGRATION_ERROR.get() { div { class: "notice", "{error}" } }
                if let Some(update) = startup_update() {
                    div { class: "notice", {language().pick("Доступно обновление Pirate Cinema ", "Pirate Cinema update available ")} "{update.version}. "
                        button { class: "secondary", onclick: move |_| page.set(Page::Settings), {language().pick("Открыть настройки", "Open settings")} }
                    }
                }
                if let Some((item, open)) = pending_duplicate() {
                    div { class: "notice",
                        {language().pick("В медиатеке уже есть другая раздача с таким названием. Добавить её всё равно? ", "The library already contains another release with this title. Add it anyway? ")}
                        button { class: "secondary", onclick: { let item = item.clone(); move |_| { pending_duplicate.set(None); add_search_result(item.clone(), open, endpoint(), page, busy, selected, files, server, cards, recent); } }, {language().pick("Добавить", "Add")} }
                        button { class: "secondary", onclick: move |_| pending_duplicate.set(None), {language().pick("Отмена", "Cancel")} }
                    }
                }
                if let Some(request) = pending_playback() {
                    div { class: "notice",
                        {language().pick("Плеер уже воспроизводит другой файл. Заменить его или открыть новое окно? ", "The player is already playing another file. Replace it or open a new window? ")}
                        button { class: "secondary", onclick: { let request = request.clone(); move |_| { players.write().clear(); web_player.set(None); pending_playback.set(None); launch_playback(request.clone(), endpoint(), busy, players, web_player, server); } }, {language().pick("Заменить", "Replace")} }
                        button { class: "secondary", onclick: { let request = request.clone(); move |_| { let mut request = request.clone(); request.force_mpv = true; pending_playback.set(None); launch_playback(request, endpoint(), busy, players, web_player, server); } }, {language().pick("Новое окно MPV", "New MPV window")} }
                        button { class: "secondary", onclick: move |_| pending_playback.set(None), {language().pick("Отмена", "Cancel")} }
                    }
                }
                if !player_labels.is_empty() {
                    div { class: "player-strip",
                        for (index, label) in player_labels.into_iter().enumerate() {
                            span { title: "{label}", "MPV · {label}" }
                            button { onclick: move |_| control_player(index, "focus", players, server), {language().pick("Показать", "Show")} }
                            button { onclick: move |_| control_player(index, "pause", players, server), {language().pick("Пауза / продолжить", "Pause / resume")} }
                            button { onclick: move |_| control_player(index, "stop", players, server), {language().pick("Закрыть", "Close")} }
                        }
                    }
                }
                if let Some(playback) = web_player() {
                    div { class: if page() == Page::Player { "web-player" } else { "web-player mini" },
                        header {
                            strong { title: "{playback.file.name}", "{playback.file.name}" }
                            if page() != Page::Player {
                                button { class: "secondary", onclick: move |_| { return_page.set(page()); page.set(Page::Player); }, {language().pick("Развернуть", "Expand")} }
                            } else {
                                button { class: "secondary", onclick: move |_| page.set(return_page()), {language().pick("Свернуть", "Minimize")} }
                            }
                            button { class: "secondary", onclick: move |_| { spawn(async move { flush_web_progress().await; web_player.set(None); if page() == Page::Player { page.set(return_page()); } }); }, {language().pick("Закрыть", "Close")} }
                        }
                        div {
                            class: "artplayer-host",
                            id: "pirate-cinema-video",
                            "data-playback-key": "{playback.torrent.hash}-{playback.file.id}",
                            "data-hls-url": "{playback.url}",
                            "data-probe-url": "{playback.probe_url}",
                            "data-heartbeat-url": "{playback.heartbeat_url}",
                            "data-title": "{playback.file.name}",
                        }
                    }
                }
                match page() {
                    Page::Home => rsx! { Home { language: language(), recent: recent(), cards: cards(), on_open: open_saved } },
                    Page::Search => rsx! { SearchPage { language: language(), query: query(), metadata: search_metadata(), poster: search_poster(), results: results(), on_add: add_result } },
                    Page::Library => rsx! { Library { language: language(), cards: cards(), busy: busy(), status: metadata_status(), on_open: open_saved, on_sync: sync_metadata } },
                    Page::Detail => rsx! { Detail { language: language(), torrent: selected(), metadata: selected.read().as_ref().and_then(|torrent| cards.read().iter().find(|card| card.torrent.hash == torrent.hash).and_then(|card| card.metadata.clone())), media_type: selected.read().as_ref().and_then(|torrent| cards.read().iter().find(|card| card.torrent.hash == torrent.hash).and_then(|card| card.media_type.clone())), poster: selected.read().as_ref().and_then(|torrent| cards.read().iter().find(|card| card.torrent.hash == torrent.hash).and_then(|card| card.poster.clone())), files: files(), busy: busy(), metadata_status: metadata_status(), on_back: move |_| page.set(Page::Library), on_play: play_file, on_media_type: update_media_type, on_title: update_title, on_refresh_metadata: refresh_selected_metadata, on_remove: remove_saved } },
                    Page::Player => rsx! { h1 { {language().pick("Просмотр", "Now playing")} } },
                    Page::Settings => rsx! { Settings { language, endpoint, busy, server, cards, recent, metadata_status, online_icon: online_icon.clone(), on_sync: sync_metadata } },
                }
            }
        } }
    }
}

struct ShutdownGuard;

struct MetadataSyncGuard;

impl Drop for MetadataSyncGuard {
    fn drop(&mut self) {
        METADATA_SYNC_ACTIVE.store(false, Ordering::Release);
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        shutdown_owned_processes();
    }
}

#[component]
fn Welcome(on_complete: EventHandler<Preferences>) -> Element {
    let initial = INITIAL_PREFERENCES
        .get()
        .cloned()
        .unwrap_or_else(default_preferences);
    let mut english = use_signal(|| initial.language == Language::English);
    let mut endpoint = use_signal(|| initial.torrserver_url);
    let mut external = use_signal(|| initial.player_type == PlayerType::External);
    let embedded_player = initial.embedded_player;
    let mut player_path = use_signal(|| initial.player_path);
    let mut error = use_signal(String::new);
    let submit = move |_| {
        let preferences = Preferences {
            language: if english() {
                Language::English
            } else {
                Language::Russian
            },
            onboarding_complete: true,
            torrserver_url: endpoint(),
            player_type: if external() {
                PlayerType::External
            } else {
                PlayerType::BundledMpv
            },
            player_path: player_path(),
            embedded_player,
            torznab_url: initial.torznab_url.clone(),
            torznab_api_key: initial.torznab_api_key.clone(),
            close_to_tray: initial.close_to_tray,
        };
        if let Err(message) =
            settings::validate_endpoint(&preferences.torrserver_url).and_then(|_| {
                settings::validate_player(preferences.player_type, &preferences.player_path)
            })
        {
            error.set(message);
        } else {
            on_complete.call(preferences);
        }
    };
    rsx! {
        main { class: "welcome-shell",
            section { class: "welcome-card",
                span { class: "eyebrow", "Pirate Cinema · Rust" }
                h1 { if english() { "Welcome" } else { "Добро пожаловать" } }
                p { class: "lead", if english() { "Choose the local service and player. You can change them later." } else { "Выберите локальный сервис и плеер. Эти параметры можно изменить позже." } }
                div { class: "filters",
                    button { class: if !english() { "primary" } else { "secondary" }, onclick: move |_| english.set(false), "Русский" }
                    button { class: if english() { "primary" } else { "secondary" }, onclick: move |_| english.set(true), "English" }
                }
                label { "TorrServer", input { value: "{endpoint}", oninput: move |event| endpoint.set(event.value()) } }
                label { input { r#type: "checkbox", checked: external(), onchange: move |event| external.set(event.checked()) } if english() { " Use an external player" } else { " Использовать внешний плеер" } }
                if external() { label { if english() { "Player path" } else { "Путь к плееру" } input { value: "{player_path}", oninput: move |event| player_path.set(event.value()) } } }
                if external() { button { class: "secondary", onclick: move |_| { if let Some(path) = rfd::FileDialog::new().add_filter("Executable", &["exe"]).pick_file() { player_path.set(path.display().to_string()); } }, if english() { "Choose player" } else { "Выбрать плеер" } } }
                if !error().is_empty() { div { class: "notice", "{error}" } }
                button { class: "primary welcome-action", onclick: submit, if english() { "Continue" } else { "Начать" } }
            }
        }
    }
}

#[component]
fn NavButton(
    label: &'static str,
    icon: String,
    active: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! { button { class: if active { "active" } else { "" }, onclick: move |event| onclick.call(event),
        if icon.is_empty() { i { class: "search-icon" } } else { img { class: "nav-icon", src: "{icon}", alt: "" } }
        span { "{label}" }
    } }
}

fn png_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn jpeg_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[component]
fn Home(
    language: Language,
    recent: Vec<ContinueItem>,
    cards: Vec<LibraryCard>,
    on_open: EventHandler<Torrent>,
) -> Element {
    let recently_played = recent
        .into_iter()
        .filter_map(|item| {
            cards
                .iter()
                .find(|card| card.torrent.hash.eq_ignore_ascii_case(&item.torrent.hash))
                .cloned()
                .map(|card| (item, card))
        })
        .collect::<Vec<_>>();
    rsx! {
        section { class: "page",
            span { class: "eyebrow", {language.pick("Локальная медиатека", "Local media library")} }
            h1 { {language.pick("Недавно запускали", "Recently played")} }
            p { class: "lead", {language.pick("Фильмы и сериалы из вашей локальной истории.", "Movies and series from your local history.")} }
            if recently_played.is_empty() {
                div { class: "panel", {language.pick("Здесь появятся фильмы и сериалы после первого запуска.", "Movies and series will appear here after the first launch.")} }
            } else {
                div { class: "home-recent",
                    for (item, card) in recently_played {
                        button { class: "card", onclick: { let torrent = item.torrent.clone(); move |_| on_open.call(torrent.clone()) },
                            div { class: "poster",
                                if let Some(poster) = card.poster { img { src: "{poster}", alt: "" } } else { "▶" }
                            }
                            strong { title: "{item.title}", "{item.title}" }
                            small { "{item.file_name} · {clock(item.position)} / {clock(item.duration)}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SearchPage(
    language: Language,
    query: String,
    metadata: Option<Movie>,
    poster: String,
    results: Vec<SearchResult>,
    on_add: EventHandler<(SearchResult, bool)>,
) -> Element {
    let mut quality = use_signal(|| "all".to_owned());
    let mut qualities = results
        .iter()
        .map(|item| release_quality(&item.title))
        .collect::<Vec<_>>();
    qualities.sort_unstable();
    qualities.dedup();
    let selected_quality = if qualities.contains(&quality().as_str()) {
        quality()
    } else {
        "all".to_owned()
    };
    let visible = results
        .into_iter()
        .filter(|item| {
            selected_quality == "all" || release_quality(&item.title) == selected_quality
        })
        .collect::<Vec<_>>();
    rsx! {
        section { class: "page search-layout",
            aside { class: "search-feature",
                if !poster.is_empty() { div { class: "poster", img { src: "{poster}", alt: "" } } }
                div {
                    span { class: "eyebrow", {language.pick("Фильм или сериал", "Movie or series")} }
                    h1 { if let Some(item) = metadata.as_ref() { "{item.title}" } else if query.is_empty() { {language.pick("Поиск", "Search")} } else { "{query}" } }
                    if let Some(item) = metadata.as_ref() {
                        p { class: "lead", "{item.year.map(|year| year.to_string()).unwrap_or_default()}" if let Some(rating) = item.rating { " · ★ {rating:.1}" } }
                        if let Some(overview) = item.overview.as_ref() { p { class: "overview", "{overview}" } }
                    } else { p { class: "lead", {language.pick("Выберите подходящую раздачу справа.", "Choose a suitable release on the right.")} } }
                }
            }
            div { class: "search-results",
                span { class: "eyebrow", {language.pick("Результаты поиска", "Search results")} }
                h2 { {language.pick("Раздачи", "Releases")} }
                if !qualities.is_empty() {
                    div { class: "filters",
                        button { class: if selected_quality == "all" { "primary" } else { "secondary" }, onclick: move |_| quality.set("all".into()), {language.pick("Все", "All")} }
                        for item in qualities {
                            button { class: if selected_quality == item { "primary" } else { "secondary" }, onclick: { let item = item.to_owned(); move |_| quality.set(item.clone()) }, if item == "Другое" { {language.pick("Другое", "Other")} } else { "{item}" } }
                        }
                    }
                }
                if visible.is_empty() { div { class: "empty", {language.pick("Введите название в строке сверху или измените фильтр.", "Enter a title above or change the filter.")} } } else { div { class: "results",
                    for item in visible {
                        article { class: "result",
                            strong { title: "{item.title}", "[{release_quality(&item.title)}] {item.title}" }
                            span { "{item.size}" }
                            small { if language == Language::Russian { "{item.seeders} сидов" } else { "{item.seeders} seeders" } }
                            button { class: "primary result-action", onclick: { let item = item.clone(); move |_| on_add.call((item.clone(), false)) }, {language.pick("Добавить", "Add")} }
                            button { class: "primary result-action", onclick: { let item = item.clone(); move |_| on_add.call((item.clone(), true)) }, {language.pick("Смотреть", "Watch")} }
                        }
                    }
                } }
            }
        }
    }
}

#[component]
fn Library(
    language: Language,
    cards: Vec<LibraryCard>,
    busy: bool,
    status: String,
    on_open: EventHandler<Torrent>,
    on_sync: EventHandler<MouseEvent>,
) -> Element {
    let mut filter = use_signal(String::new);
    let mut kind = use_signal(|| "all".to_owned());
    let mut watched = use_signal(|| "all".to_owned());
    let mut genre = use_signal(|| "all".to_owned());
    let mut year = use_signal(|| "all".to_owned());
    let mut order = use_signal(|| "recent".to_owned());
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
    let mut visible = cards.clone();
    let needle = filter().trim().to_lowercase();
    visible.retain(|card| {
        let title = card
            .metadata
            .as_ref()
            .map_or(card.torrent.title.as_str(), |item| item.title.as_str());
        (needle.is_empty() || title.to_lowercase().contains(&needle))
            && (kind() == "all" || card.media_type.as_deref() == Some(kind().as_str()))
            && (genre() == "all"
                || card
                    .metadata
                    .as_ref()
                    .is_some_and(|item| item.genres.contains(&genre())))
            && (year() == "all"
                || card
                    .metadata
                    .as_ref()
                    .and_then(|item| item.year)
                    .map(|value| value.to_string())
                    .as_deref()
                    == Some(year().as_str()))
            && match watched().as_str() {
                "viewed" => card.viewed,
                "unviewed" => !card.viewed,
                _ => true,
            }
    });
    match order().as_str() {
        "title" => visible.sort_by_key(|card| {
            card.metadata
                .as_ref()
                .map_or(card.torrent.title.as_str(), |item| item.title.as_str())
                .to_lowercase()
        }),
        "year" => visible.sort_by_key(|card| {
            std::cmp::Reverse(
                card.metadata
                    .as_ref()
                    .and_then(|item| item.year)
                    .unwrap_or(0),
            )
        }),
        _ => {}
    }
    rsx! {
        section { class: "page",
            span { class: "eyebrow", {language.pick("Сохранено в TorrServer", "Saved in TorrServer")} }
            div { class: "detail-head",
                h1 { {language.pick("Медиатека", "Library")} }
                button { class: "secondary", disabled: busy, onclick: move |event| on_sync.call(event), if busy { {language.pick("Обновляем…", "Updating…")} } else { {language.pick("Обновить описания и постеры", "Refresh descriptions and posters")} } }
            }
            p { class: "lead", if language == Language::Russian { "Раздач: {cards.len()}. Метаданные, постеры и история хранятся в локальной SQLite." } else { "Items: {cards.len()}. Metadata, posters, and history are stored in local SQLite." } }
            div { class: "filters",
                input { value: "{filter}", placeholder: language.pick("Фильтр по названию", "Filter by title"), oninput: move |event| filter.set(event.value()) }
                select { value: "{kind}", onchange: move |event| kind.set(event.value()), option { value: "all", {language.pick("Все типы", "All types")} } option { value: "movie", {language.pick("Фильмы", "Movies")} } option { value: "series", {language.pick("Сериалы", "Series")} } }
                select { value: "{watched}", onchange: move |event| watched.set(event.value()), option { value: "all", {language.pick("Любой статус", "Any status")} } option { value: "viewed", {language.pick("Просмотрено", "Viewed")} } option { value: "unviewed", {language.pick("Не просмотрено", "Not viewed")} } }
                select { value: "{genre}", onchange: move |event| genre.set(event.value()), option { value: "all", {language.pick("Любой жанр", "Any genre")} } for item in genres { option { value: "{item}", "{item}" } } }
                select { value: "{year}", onchange: move |event| year.set(event.value()), option { value: "all", {language.pick("Любой год", "Any year")} } for item in years { option { value: "{item}", "{item}" } } }
                select { value: "{order}", onchange: move |event| order.set(event.value()), option { value: "recent", {language.pick("Недавно добавленные", "Recently added")} } option { value: "title", {language.pick("По названию", "By title")} } option { value: "year", {language.pick("По году", "By year")} } }
            }
            if !status.is_empty() { div { class: "notice", "{status}" } }
            if visible.is_empty() {
                div { class: "empty", {language.pick("Медиатека пока пуста.", "The library is empty.")} }
            } else {
                div { class: "grid",
                    for card in visible {
                        article { class: "card", title: "{card.torrent.title}",
                            button { class: "card-open", onclick: { let torrent = card.torrent.clone(); move |_| on_open.call(torrent.clone()) },
                            div { class: "poster", if let Some(poster) = card.poster.as_ref() { img { src: "{poster}", alt: "" } } else { "▶" } }
                            strong { "{card.metadata.as_ref().map(|item| item.title.as_str()).unwrap_or(&card.torrent.title)}" }
                            small { class: if card.viewed { "viewed" } else { "" },
                                if card.viewed { {language.pick("✓ Просмотрено", "✓ Viewed")} } else if let Some(year) = card.metadata.as_ref().and_then(|item| item.year) { "{year}" } else { "BTIH · {short_hash(&card.torrent.hash)}" }
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
fn Detail(
    language: Language,
    torrent: Option<Torrent>,
    metadata: Option<MediaMetadata>,
    media_type: Option<String>,
    poster: Option<String>,
    files: Vec<PlayableFile>,
    busy: bool,
    metadata_status: String,
    on_back: EventHandler<MouseEvent>,
    on_play: EventHandler<(VideoFile, bool, bool)>,
    on_media_type: EventHandler<String>,
    on_title: EventHandler<String>,
    on_refresh_metadata: EventHandler<MouseEvent>,
    on_remove: EventHandler<MouseEvent>,
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
        return rsx! { section { class: "page", div { class: "empty", {language.pick("Раздача не выбрана", "No torrent selected")} } } };
    };
    let mut title_edit = use_signal(|| {
        metadata
            .as_ref()
            .map_or_else(|| torrent.title.clone(), |item| item.title.clone())
    });
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
        .into_iter()
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
        .collect::<Vec<_>>();
    visible_files.sort_by_key(|item| {
        (
            is_episode(&item.file.path),
            file_season(&item.file.path).unwrap_or(0),
            file_episode(&item.file.path).unwrap_or(0),
            item.file.path.to_lowercase(),
        )
    });
    rsx! {
        section { class: "page",
            div { class: "detail-layout",
                div { if let Some(poster) = poster { img { class: "detail-cover", src: "{poster}", alt: "" } } else { div { class: "detail-cover poster", "▶" } } }
                div {
                    div { class: "detail-head",
                        div { span { class: "eyebrow", "BTIH · {short_hash(&torrent.hash)}" } h1 { "{metadata.as_ref().map(|item| item.title.as_str()).unwrap_or(&torrent.title)}" } }
                        button { class: "secondary", onclick: move |event| on_back.call(event), {language.pick("Назад к медиатеке", "Back to library")} }
                    }
                    if let Some(item) = metadata.as_ref() {
                        p { class: "lead",
                            if let Some(year) = item.year { "{year}" }
                            if let Some(rating) = item.rating { " · ★ {rating:.1}" }
                        }
                        if let Some(overview) = item.overview.as_ref() { p { class: "overview", "{overview}" } }
                    } else { p { class: "lead", {language.pick("Локальное описание ещё не загружено.", "The local description has not been loaded yet.")} } }
                    div { class: "filters",
                        button { class: if media_type.as_deref() == Some("movie") { "primary" } else { "secondary" }, onclick: move |_| on_media_type.call("movie".into()), {language.pick("Фильм", "Movie")} }
                        button { class: if media_type.as_deref() == Some("series") { "primary" } else { "secondary" }, onclick: move |_| on_media_type.call("series".into()), {language.pick("Сериал", "Series")} }
                    }
                    div { class: "filters",
                        input { value: "{title_edit}", placeholder: language.pick("Название в медиатеке", "Library title"), oninput: move |event| title_edit.set(event.value()) }
                        button { class: "secondary", onclick: move |_| on_title.call(title_edit()), {language.pick("Сохранить название", "Save title")} }
                        button { class: "secondary", disabled: busy, onclick: move |event| on_refresh_metadata.call(event), {language.pick("Обновить описание и постер", "Refresh description and poster")} }
                    }
                    if !metadata_status.is_empty() { div { class: "notice", "{metadata_status}" } }
                    if confirm_remove() {
                        div { class: "notice",
                            {language.pick("Удалить раздачу из TorrServer? История просмотра останется локально. ", "Remove this torrent from TorrServer? Playback history will remain local. ")}
                            button { class: "secondary", onclick: move |event| on_remove.call(event), {language.pick("Удалить", "Remove")} }
                            button { class: "secondary", onclick: move |_| confirm_remove.set(false), {language.pick("Отмена", "Cancel")} }
                        }
                    } else {
                        button { class: "secondary", onclick: move |_| confirm_remove.set(true), {language.pick("Удалить из TorrServer", "Remove from TorrServer")} }
                    }
                }
            }
            h2 { {language.pick("Выберите файл", "Choose a file")} }
            if has_next_file { label { class: "auto-next", input { r#type: "checkbox", checked: auto_next(), onchange: { let hash = torrent.hash.clone(); move |event| { auto_next.set(event.checked()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } } } {language.pick(" Автопереход к следующей серии", " Play the next episode automatically")} } }
            if media_type.as_deref() == Some("series") && !season_progress.is_empty() {
                div { class: "season-progress",
                    for (number, watched, total) in season_progress {
                        span { if number == 0 { {language.pick("Спецвыпуски", "Specials")} } else if language == Language::Russian { "Сезон {number}" } else { "Season {number}" } " · {watched}/{total}" }
                    }
                }
            }
            if media_type.as_deref() == Some("series") {
                if let Some(next) = visible_files.iter().find(|item| !item.viewed) {
                    button { class: "primary", style: "min-height:42px; margin-bottom:16px", onclick: { let file = next.file.clone(); move |_| on_play.call((file.clone(), true, auto_next())) }, if language == Language::Russian { "Следующая непросмотренная: {next.file.name}" } else { "Next unwatched: {next.file.name}" } }
                }
            }
            if visible_files.len() > 1 || !needle.is_empty() || !seasons.is_empty() {
                div { class: "filters",
                    input { value: "{file_filter}", placeholder: language.pick("Найти фильм или серию", "Find a movie or episode"), oninput: move |event| file_filter.set(event.value()) }
                    if has_movies && has_episodes {
                        select { value: "{file_kind}", onchange: move |event| { file_kind.set(event.value()); season.set("all".into()); episode.set("all".into()); },
                            option { value: "all", {language.pick("Фильмы и серии", "Movies and episodes")} }
                            option { value: "movies", {language.pick("Только фильмы", "Movies only")} }
                            option { value: "episodes", {language.pick("Только серии", "Episodes only")} }
                        }
                    }
                    if seasons.len() > 1 {
                        select { value: "{season}", onchange: { let hash = torrent.hash.clone(); move |event| { season.set(event.value()); episode.set("all".into()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } },
                            option { value: "all", {language.pick("Все сезоны", "All seasons")} }
                            for number in seasons { option { value: "{number}", if number == 0 { {language.pick("Спецвыпуски", "Specials")} } else if language == Language::Russian { "Сезон {number}" } else { "Season {number}" } } }
                        }
                    }
                    if !episodes.is_empty() {
                        select { value: "{episode}", onchange: { let hash = torrent.hash.clone(); move |event| { episode.set(event.value()); save_detail_preferences(&hash, &season(), &episode(), auto_next()); } },
                            option { value: "all", {language.pick("Все серии", "All episodes")} }
                            for number in episodes { option { value: "{number}", if language == Language::Russian { "Серия {number}" } else { "Episode {number}" } } }
                        }
                    }
                }
            }
            if busy && files_empty {
                div { class: "panel", {language.pick("Получаем список файлов…", "Loading the file list…")} }
            } else if files_empty {
                div { class: "empty", {language.pick("Видеофайлы пока недоступны. Повторно откройте карточку через несколько секунд.", "Video files are not available yet. Reopen the card in a few seconds.")} }
            } else {
                div { class: "files",
                    for item in visible_files {
                        article { class: "file",
                            div { strong { title: "{item.file.path}", "{item.file.name}" } small { if item.position > 0 { if language == Language::Russian { "Сохранено: {clock(item.position)} / {clock(item.duration)}" } else { "Saved: {clock(item.position)} / {clock(item.duration)}" } } else if item.viewed { {language.pick("Просмотрено", "Viewed")} } else { {language.pick("Не запускался", "Not started")} } } }
                            button { class: "primary launch-button", onclick: { let file = item.file.clone(); move |_| on_play.call((file.clone(), true, auto_next())) }, {language.pick("Запустить", "Play")} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Settings(
    mut language: Signal<Language>,
    mut endpoint: Signal<String>,
    busy: Signal<bool>,
    server: Signal<ServerState>,
    mut cards: Signal<Vec<LibraryCard>>,
    mut recent: Signal<Vec<ContinueItem>>,
    metadata_status: Signal<String>,
    online_icon: String,
    on_sync: EventHandler<MouseEvent>,
) -> Element {
    let initial = INITIAL_PREFERENCES
        .get()
        .cloned()
        .unwrap_or_else(default_preferences);
    let initial_endpoint = endpoint();
    let mut endpoint_edit = use_signal(move || initial_endpoint);
    let mut torznab_url = use_signal(|| initial.torznab_url);
    let mut torznab_api_key = use_signal(|| initial.torznab_api_key);
    let mut player_path = use_signal(|| initial.player_path);
    let mut external_player = use_signal(|| initial.player_type == PlayerType::External);
    let mut embedded_player = use_signal(|| initial.embedded_player);
    let mut close_to_tray = use_signal(|| initial.close_to_tray);
    let mut status = use_signal(String::new);
    let mut maintenance_busy = use_signal(|| false);
    let mut diagnostics = use_signal(Vec::<String>::new);
    let mut backup_status = use_signal(String::new);
    let mut restore_source = use_signal(|| None::<PathBuf>);
    let mut update_status = use_signal(String::new);
    let mut update_url = use_signal(String::new);
    let mut available_update = use_signal(|| None::<ReleaseUpdate>);
    let window = dioxus::desktop::use_window();
    let exit_fullscreen = move |_| {
        window.set_fullscreen(false);
        spawn(async move {
            let mut eval = document::eval(
                r#"
                const art = window.__pirateCinemaArt;
                if (art) { art.fullscreen = false; art.fullscreenWeb = false; }
                if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
                dioxus.send(true);
            "#,
            );
            let _ = eval.recv::<bool>().await;
        });
    };

    let save = move |_| {
        let next = Preferences {
            language: language(),
            onboarding_complete: true,
            torrserver_url: endpoint_edit(),
            player_type: if external_player() {
                PlayerType::External
            } else {
                PlayerType::BundledMpv
            },
            player_path: player_path(),
            embedded_player: embedded_player(),
            torznab_url: torznab_url(),
            torznab_api_key: torznab_api_key(),
            close_to_tray: close_to_tray(),
        };
        let result = settings_path().and_then(|path| settings::save_preferences(&path, &next));
        if let Err(error) = result {
            status.set(error);
            return;
        }
        if let Err(error) = configure_torznab(
            next.torrserver_url.as_str(),
            next.torznab_url.as_str(),
            next.torznab_api_key.as_str(),
        ) {
            status.set(error);
            return;
        }
        endpoint.set(next.torrserver_url);
        status.set(
            language()
                .pick("Настройки сохранены", "Settings saved")
                .into(),
        );
        refresh_server(endpoint(), busy, server, cards, recent);
    };

    let run_diagnostics = move |_| {
        if maintenance_busy() {
            return;
        }
        maintenance_busy.set(true);
        diagnostics.set(vec!["Выполняем проверки…".into()]);
        let endpoint_value = endpoint_edit();
        let selected_player = if external_player() {
            PlayerType::External
        } else {
            PlayerType::BundledMpv
        };
        let selected_path = player_path();
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                system_diagnostics(&endpoint_value, selected_player, &selected_path)
            })
            .await;
            diagnostics.set(match result {
                Ok(items) => items,
                Err(error) => vec![format!("✕ Диагностика: {error}")],
            });
            maintenance_busy.set(false);
        });
    };

    let create_backup = move |_| {
        if maintenance_busy() {
            return;
        }
        let Some(directory) = rfd::FileDialog::new()
            .set_title("Папка для резервной копии Pirate Cinema")
            .pick_folder()
        else {
            return;
        };
        let endpoint_value = endpoint();
        maintenance_busy.set(true);
        backup_status.set("Создаём резервную копию…".into());
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                while_owned_torrserver_stopped(&endpoint_value, |torrserver_dir| {
                    create_local_backup(
                        &history_path()?,
                        settings_path().ok().as_deref(),
                        torrserver_dir,
                        &directory,
                    )
                })
            })
            .await;
            backup_status.set(match result {
                Ok(Ok(path)) => format!("Копия создана: {}", path.display()),
                Ok(Err(error)) => error,
                Err(error) => error.to_string(),
            });
            maintenance_busy.set(false);
        });
    };

    let choose_restore = move |_| {
        if maintenance_busy() {
            return;
        }
        restore_source.set(
            rfd::FileDialog::new()
                .set_title("Выберите папку backup-* для восстановления")
                .pick_folder(),
        );
    };

    let restore_backup = move |_| {
        let Some(source) = restore_source() else {
            return;
        };
        let endpoint_value = endpoint();
        maintenance_busy.set(true);
        backup_status.set("Проверяем копию и создаём страховочную…".into());
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                while_owned_torrserver_stopped(&endpoint_value, |torrserver_dir| {
                    let history_file = history_path()?;
                    let profile = history_file
                        .parent()
                        .ok_or("Папка профиля Rust недоступна")?
                        .to_path_buf();
                    let preferences = settings_path().ok();
                    let safety = create_local_backup(
                        &history_file,
                        preferences.as_deref(),
                        torrserver_dir,
                        &profile.join("pre-restore-backups"),
                    )?;
                    let mut history =
                        HistoryStore::open(&history_file).map_err(|error| error.to_string())?;
                    let posters = history.restore_local_backup(
                        &source,
                        preferences.as_deref(),
                        &profile,
                        torrserver_dir,
                    )?;
                    Ok::<_, String>((posters, safety))
                })
            })
            .await;
            backup_status.set(match result {
                Ok(Ok((posters, safety))) => {
                    cards.set(load_library_cards(&server.read().torrents).unwrap_or_default());
                    recent.set(load_continue_items(&server.read().torrents).unwrap_or_default());
                    restore_source.set(None);
                    format!(
                        "Копия восстановлена · постеров: {posters}. Страховочная копия: {}. Перезапустите приложение для применения настроек",
                        safety.display()
                    )
                }
                Ok(Err(error)) => format!("Восстановление отменено: {error}"),
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
        update_status.set("Проверяем GitHub Releases…".into());
        update_url.set(String::new());
        available_update.set(None);
        spawn(async move {
            let result = tokio::task::spawn_blocking(|| check_rust_update(VERSION)).await;
            match result {
                Ok(Ok(Some(update))) => {
                    update_url.set(update.download_url.clone());
                    update_status.set(format!("Доступна версия {}", update.version));
                    available_update.set(Some(update));
                }
                Ok(Ok(None)) => update_status.set("Установлена актуальная версия".into()),
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
        update_status.set(format!("Скачиваем версию {}…", update.version));
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let installer = download_rust_update(&update)?;
                launch_update_installer(&installer)?;
                Ok::<_, String>(installer)
            })
            .await;
            match result {
                Ok(Ok(installer)) => update_status.set(format!(
                    "Установщик запущен: {}. Закройте Pirate Cinema, если установщик попросит.",
                    installer.display()
                )),
                Ok(Err(error)) => update_status.set(error),
                Err(error) => update_status.set(error.to_string()),
            }
            maintenance_busy.set(false);
        });
    };

    rsx! {
        section { class: "page settings",
            span { class: "eyebrow", {language().pick("Локальная конфигурация", "Local configuration")} }
            h1 { {language().pick("Настройки", "Settings")} }
            p { class: "lead", "Pirate Cinema · {VERSION}. " {language().pick("Настройки и данные хранятся только на этом компьютере.", "Preferences and data stay on this computer.")} }

            section { class: "settings-card",
                h2 { {language().pick("Интерфейс и плеер", "Interface and player")} }
                div { class: "setting-row",
                    span { {language().pick("Язык", "Language")} }
                    select { value: if language() == Language::English { "en" } else { "ru" }, onchange: move |event| language.set(if event.value() == "en" { Language::English } else { Language::Russian }),
                        option { value: "ru", "Русский" }
                        option { value: "en", "English" }
                    }
                }
                div { class: "choice-row",
                    button { class: if !external_player() { "secondary selected" } else { "secondary" }, onclick: move |_| external_player.set(false), {language().pick("Плеер приложения", "Application player")} }
                    button { class: if external_player() { "secondary selected" } else { "secondary" }, onclick: move |_| external_player.set(true), {language().pick("Локальный плеер", "Local player")} }
                }
                if !external_player() {
                    label { class: "auto-next",
                        input { r#type: "checkbox", checked: embedded_player(), onchange: move |event| embedded_player.set(event.checked()) }
                        {language().pick(" Воспроизводить внутри приложения через HLS", " Play inside the application over HLS")}
                    }
                }
                button { class: "secondary", onclick: exit_fullscreen, {language().pick("Выйти из полноэкранного режима", "Exit fullscreen")} }
                div { class: "setting-row",
                    span { {language().pick("При закрытии", "When closing")} }
                    div { class: "stacked-options",
                        label { class: "auto-next",
                            input { r#type: "checkbox", checked: close_to_tray(), onchange: move |event| close_to_tray.set(event.checked()) }
                            {language().pick(" Сворачивать в трей", " Minimize to tray")}
                        }
                        label { class: "auto-next",
                            input { r#type: "checkbox", checked: !close_to_tray(), onchange: move |event| close_to_tray.set(!event.checked()) }
                            {language().pick(" Закрывать приложение полностью", " Exit the application completely")}
                        }
                    }
                }
                p { class: "hint", {language().pick("Варианты взаимоисключающие; настройка применяется после перезапуска.", "These options are mutually exclusive; the setting applies after restart.")} }
                p { class: "hint", {language().pick("Только встроенный MPV сохраняет точную позицию просмотра через IPC.", "Only bundled MPV saves exact playback position through IPC.")} }
                if external_player() {
                    div { class: "setting-row",
                        span { {language().pick("Плеер", "Player")} }
                        div { class: "choice-row",
                            input { value: "{player_path}", placeholder: "C:\\Program Files\\mpv\\mpv.exe", oninput: move |event| player_path.set(event.value()) }
                            button { class: "secondary", onclick: move |_| { if let Some(path) = rfd::FileDialog::new().add_filter("Executable", &["exe"]).pick_file() { player_path.set(path.display().to_string()); } }, {language().pick("Выбрать файл", "Choose file")} }
                        }
                    }
                }
            }

            section { class: "settings-card",
                h2 { {language().pick("TorrServer и поиск", "TorrServer and search")} }
                p { class: "hint", {language().pick("Локальный сервер и необязательный индексатор Jackett/Prowlarr.", "Local server and optional Jackett/Prowlarr indexer.")} }
                label { {language().pick("Адрес TorrServer", "TorrServer address")} input { value: "{endpoint_edit}", oninput: move |event| endpoint_edit.set(event.value()) } }
                label { {language().pick("Адрес Jackett/Torznab", "Jackett/Torznab address")} input { value: "{torznab_url}", placeholder: "http://127.0.0.1:9117/api/v2.0/indexers/all/results/torznab/", oninput: move |event| torznab_url.set(event.value()) } }
                label { {language().pick("API-ключ Jackett", "Jackett API key")} input { r#type: "password", value: "{torznab_api_key}", placeholder: language().pick("Оставьте пустым, чтобы отключить", "Leave empty to disable"), oninput: move |event| torznab_api_key.set(event.value()) } }
                button { class: "primary", onclick: save, {language().pick("Сохранить и применить", "Save and apply")} }
                if !status().is_empty() { div { class: "notice", "{status}" } }
            }

            section { class: "settings-card",
                h2 { {language().pick("Обслуживание", "Maintenance")} }
                p { class: "hint", {language().pick("Резервная копия включает историю, настройки, постеры и данные встроенного TorrServer.", "Backup includes history, preferences, posters, and bundled TorrServer data.")} }
                div { class: "settings-facts",
                    article {
                        span { {language().pick("Поиск раздач", "Torrent search")} }
                        strong { if torznab_url().trim().is_empty() { "RuTor" } else { "RuTor · Torznab" } }
                        small { {language().pick("Через локальный TorrServer", "Through local TorrServer")} }
                    }
                    article {
                        span { {language().pick("Постеры и метаданные", "Posters and metadata")} }
                        strong { "Cinemeta · TVmaze · Wikidata" }
                        small { {language().pick("Локальный кэш и кадр MPV", "Local cache and MPV frame")} }
                    }
                    article {
                        span { {language().pick("Локальная база", "Local database")} }
                        strong { "SQLite · media.db" }
                        small { if language() == Language::Russian { "Карточек: {cards().len()}" } else { "Items: {cards().len()}" } }
                    }
                }
                button { class: "primary", disabled: busy(), onclick: move |event| on_sync.call(event), if busy() { {language().pick("Синхронизируем…", "Synchronizing…")} } else { {language().pick("Синхронизировать всю медиатеку", "Synchronize entire library")} } }
                if !metadata_status().is_empty() { div { class: "notice", "{metadata_status}" } }
                div { class: "settings-actions",
                    button { class: "secondary", disabled: maintenance_busy(), onclick: run_diagnostics, {language().pick("Запустить диагностику", "Run diagnostics")} }
                    button { class: "secondary", disabled: maintenance_busy(), onclick: create_backup, {language().pick("Создать резервную копию", "Create backup")} }
                    button { class: "secondary", disabled: maintenance_busy(), onclick: choose_restore, {language().pick("Восстановить копию", "Restore backup")} }
                    button { class: "secondary", disabled: maintenance_busy(), onclick: check_update, {language().pick("Проверить обновления", "Check for updates")} }
                }
                div { class: "settings-output",
                    if !diagnostics().is_empty() { div { class: "panel", for item in diagnostics() { div { "{item}" } } } }
                    if !backup_status().is_empty() { div { class: "notice", "{backup_status}" } }
                    if !update_status().is_empty() {
                        div { class: "notice", "{update_status}"
                            if available_update().is_some() { button { class: "primary", style: "min-height:38px;margin-left:10px", disabled: maintenance_busy(), onclick: install_update, {language().pick("Скачать и установить", "Download and install")} } }
                            if !update_url().is_empty() { button { class: "secondary", style: "margin-left:8px", onclick: move |_| { if let Err(error) = open_external_url(&update_url()) { update_status.set(error); } }, {language().pick("Открыть страницу", "Open page")} } }
                        }
                    }
                    if let Some(path) = restore_source() {
                        div { class: "notice",
                            if language() == Language::Russian { "Будут восстановлены локальные история, настройки, постеры и данные встроенного TorrServer из: {path.display()}. Перед изменением автоматически создаётся страховочная копия. " } else { "Local history, settings, posters, and bundled TorrServer data will be restored from: {path.display()}. A safety backup is created first. " }
                            button { class: "secondary", disabled: maintenance_busy(), onclick: restore_backup, {language().pick("Подтвердить восстановление", "Confirm restore")} }
                            button { class: "secondary", onclick: move |_| restore_source.set(None), {language().pick("Отмена", "Cancel")} }
                        }
                    }
                }
            }

            section { class: "settings-card server-summary",
                img { src: "{online_icon}", alt: "" }
                div {
                    h2 { "TorrServer" }
                    strong { "{endpoint_edit}" }
                    if !server().version.is_empty() {
                        small { if language() == Language::Russian { "Подключён · {server().version} · раздач: {server().torrents.len()}" } else { "Connected · {server().version} · torrents: {server().torrents.len()}" } }
                    } else if server().error.is_empty() {
                        small { {language().pick("Проверяем подключение…", "Checking connection…")} }
                    } else {
                        small { {language().pick("Нет подключения — запустите диагностику выше", "Not connected — run diagnostics above")} }
                    }
                }
            }
        }
    }
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(12).collect()
}

fn release_quality(title: &str) -> &'static str {
    let title = title.to_ascii_lowercase();
    if title.contains("2160p") || title.contains("4k") || title.contains("uhd") {
        "2160p"
    } else if title.contains("1080p") || title.contains("1080i") {
        "1080p"
    } else if title.contains("720p") {
        "720p"
    } else {
        "Другое"
    }
}

fn refresh_server(
    endpoint: String,
    mut busy: Signal<bool>,
    mut server: Signal<ServerState>,
    mut cards: Signal<Vec<LibraryCard>>,
    mut recent: Signal<Vec<ContinueItem>>,
) {
    busy.set(true);
    spawn(async move {
        let request_endpoint = endpoint.clone();
        let check_gst = cfg!(windows)
            && settings_path()
                .ok()
                .and_then(|path| settings::load_preferences(&path).ok())
                .or_else(|| INITIAL_PREFERENCES.get().cloned())
                .is_some_and(|preferences| {
                    preferences.embedded_player && preferences.player_type != PlayerType::External
                });
        let result = tokio::task::spawn_blocking(move || {
            read_torrserver(&request_endpoint).map(|(version, torrents)| {
                let gst_error = if check_gst {
                    ensure_gstreamer(&request_endpoint).err()
                } else {
                    None
                };
                (version, torrents, gst_error)
            })
        })
        .await;
        server.set(match result {
            Ok(Ok((version, torrents, gst_error))) => {
                cards.set(load_library_cards(&torrents).unwrap_or_default());
                recent.set(load_continue_items(&torrents).unwrap_or_default());
                ServerState {
                    version,
                    torrents,
                    error: gst_error
                        .map(|error| format!("Встроенный HLS-плеер: {error}"))
                        .unwrap_or_default(),
                }
            }
            Ok(Err(error)) => ServerState {
                version: String::new(),
                torrents: Vec::new(),
                error: STARTUP_TORRSERVER_ERROR.get().cloned().unwrap_or(error),
            },
            Err(error) => ServerState {
                version: String::new(),
                torrents: Vec::new(),
                error: error.to_string(),
            },
        });
        busy.set(false);
    });
}

#[allow(clippy::too_many_arguments)]
fn start_search(
    value: String,
    endpoint: String,
    mut page: Signal<Page>,
    mut busy: Signal<bool>,
    mut results: Signal<Vec<SearchResult>>,
    mut metadata: Signal<Option<Movie>>,
    mut poster: Signal<String>,
    mut server: Signal<ServerState>,
) {
    page.set(Page::Search);
    busy.set(true);
    metadata.set(None);
    poster.set(String::new());
    spawn(async move {
        let metadata_query = value.clone();
        let request_endpoint = endpoint.clone();
        let result =
            tokio::task::spawn_blocking(move || search_all_sources(&request_endpoint, "", &value))
                .await;
        match result {
            Ok(Ok(found)) => {
                results.set(found);
                server.write().error.clear();
            }
            Ok(Err(error)) => {
                results.set(Vec::new());
                server.write().error = error;
            }
            Err(error) => {
                results.set(Vec::new());
                server.write().error = error.to_string();
            }
        }
        let lookup =
            tokio::task::spawn_blocking(move || match metadata::lookup(&metadata_query, false) {
                Ok(Some(item)) => Ok(Some(item)),
                Ok(None) | Err(_) => metadata::lookup(&metadata_query, true),
            })
            .await;
        if let Ok(Ok(Some(item))) = lookup {
            let poster_item = item.clone();
            metadata.set(Some(item));
            if let Ok(Ok(bytes)) = tokio::task::spawn_blocking(move || {
                metadata::movie_poster_jpeg(&poster_item, false)
            })
            .await
            {
                poster.set(jpeg_data_uri(&bytes));
            }
        }
        busy.set(false);
    });
}

#[allow(clippy::too_many_arguments)]
fn add_search_result(
    item: SearchResult,
    open: bool,
    endpoint: String,
    page: Signal<Page>,
    mut busy: Signal<bool>,
    selected: Signal<Option<Torrent>>,
    files: Signal<Vec<PlayableFile>>,
    mut server: Signal<ServerState>,
    mut cards: Signal<Vec<LibraryCard>>,
    mut recent: Signal<Vec<ContinueItem>>,
) {
    busy.set(true);
    spawn(async move {
        let title = item.title.clone();
        let link = item.magnet.clone();
        let request_endpoint = endpoint.clone();
        let added =
            tokio::task::spawn_blocking(move || add_magnet(&request_endpoint, &link, &title)).await;
        match added {
            Ok(Ok(added)) => {
                let torrent = Torrent {
                    title: item.title,
                    hash: added.hash,
                };
                let metadata_endpoint = endpoint.clone();
                let metadata_torrent = torrent.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    initialize_added_torrent(&metadata_endpoint, &metadata_torrent)
                })
                .await;
                if !server
                    .read()
                    .torrents
                    .iter()
                    .any(|saved| saved.hash.eq_ignore_ascii_case(&torrent.hash))
                {
                    server.write().torrents.push(torrent.clone());
                }
                cards.set(load_library_cards(&server.read().torrents).unwrap_or_default());
                recent.set(load_continue_items(&server.read().torrents).unwrap_or_default());
                if open {
                    open_torrent(torrent, endpoint, page, busy, selected, files, server);
                } else {
                    server.write().error = "Раздача добавлена в TorrServer".into();
                    refresh_server(endpoint, busy, server, cards, recent);
                }
            }
            Ok(Err(error)) => {
                server.write().error = error;
                busy.set(false);
            }
            Err(error) => {
                server.write().error = error.to_string();
                busy.set(false);
            }
        }
    });
}

fn open_torrent(
    torrent: Torrent,
    endpoint: String,
    mut page: Signal<Page>,
    mut busy: Signal<bool>,
    mut selected: Signal<Option<Torrent>>,
    mut files: Signal<Vec<PlayableFile>>,
    mut server: Signal<ServerState>,
) {
    selected.set(Some(torrent.clone()));
    files.set(Vec::new());
    page.set(Page::Detail);
    busy.set(true);
    spawn(async move {
        let hash = torrent.hash.clone();
        let result =
            tokio::task::spawn_blocking(move || torrent_video_files(&endpoint, &hash)).await;
        match result {
            Ok(Ok(found)) => {
                files.set(load_file_history(&torrent.hash, found));
                server.write().error.clear();
            }
            Ok(Err(error)) => server.write().error = error,
            Err(error) => server.write().error = error.to_string(),
        }
        busy.set(false);
    });
}

fn history_path() -> Result<PathBuf, String> {
    let data = default_data_dir()?;
    let root = data.parent().ok_or("Не удалось определить папку истории")?;
    std::fs::create_dir_all(root).map_err(|error| error.to_string())?;
    Ok(root.join("history.sqlite"))
}

fn save_detail_preferences(hash: &str, season: &str, episode: &str, auto_next: bool) {
    let preferences = PlaybackPreferences {
        season: season.parse().ok(),
        episode: episode.parse().ok(),
        auto_next,
    };
    if let Ok(path) = history_path() {
        if let Ok(history) = HistoryStore::open(&path) {
            let _ = history.save_playback_preferences(hash, &preferences);
        }
    }
}

fn initialize_added_torrent(endpoint: &str, torrent: &Torrent) -> Result<(), String> {
    let files = torrent_video_files(endpoint, &torrent.hash)?;
    let series = infer_series(&torrent.title, &files);
    let history = HistoryStore::open(&history_path()?).map_err(|error| error.to_string())?;
    history
        .set_media_type(&torrent.hash, if series { "series" } else { "movie" })
        .map_err(|error| error.to_string())?;
    let _ = sync_library_metadata(endpoint, std::slice::from_ref(torrent), None);
    Ok(())
}

fn settings_path() -> Result<PathBuf, String> {
    let data = default_data_dir()?;
    let root = data
        .parent()
        .ok_or("Не удалось определить папку настроек")?;
    Ok(root.join("settings.json"))
}

fn while_owned_torrserver_stopped<T>(
    endpoint: &str,
    operation: impl FnOnce(Option<&std::path::Path>) -> Result<T, String>,
) -> Result<T, String> {
    let process_lock = TORRSERVER_PROCESS
        .get()
        .ok_or("Состояние TorrServer ещё не инициализировано")?;
    let owned = process_lock
        .lock()
        .map_err(|_| "Состояние TorrServer повреждено")?
        .as_ref()
        .is_some_and(TorrServerProcess::owns_process);
    if !owned {
        return operation(None);
    }

    let process = process_lock
        .lock()
        .map_err(|_| "Состояние TorrServer повреждено")?
        .take();
    drop(process);
    let data_dir = default_data_dir()?;
    let result = operation(Some(&data_dir));
    let restart = bundled_torrserver().and_then(|executable| {
        TorrServerProcess::connect_or_start(endpoint, &executable, &data_dir)
    });
    match restart {
        Ok(process) => {
            *process_lock
                .lock()
                .map_err(|_| "Состояние TorrServer повреждено")? = Some(process);
            result
        }
        Err(error) => Err(match result {
            Ok(_) => format!("Копия создана, но TorrServer не перезапустился: {error}"),
            Err(operation_error) => {
                format!("{operation_error}; TorrServer не перезапустился: {error}")
            }
        }),
    }
}

fn default_preferences() -> Preferences {
    Preferences {
        language: Language::Russian,
        onboarding_complete: false,
        torrserver_url: DEFAULT_TORRSERVER_URL.to_owned(),
        player_type: PlayerType::BundledMpv,
        player_path: String::new(),
        embedded_player: cfg!(windows),
        torznab_url: String::new(),
        torznab_api_key: String::new(),
        close_to_tray: true,
    }
}

fn shutdown_owned_processes() {
    SHUTDOWN_REQUESTED.store(true, Ordering::Release);
    if let Some(state) = TORRSERVER_PROCESS.get() {
        if let Ok(mut owned) = state.lock() {
            // Taking the value runs TorrServerProcess::drop, which kills and waits
            // for the bundled child instead of leaving it behind after the UI exits.
            owned.take();
        }
    }
}

fn system_diagnostics(endpoint: &str, player_type: PlayerType, player_path: &str) -> Vec<String> {
    let mut checks = Vec::new();
    checks.push(match read_torrserver(endpoint) {
        Ok((version, torrents)) => {
            format!("✓ TorrServer: {version} · раздач: {}", torrents.len())
        }
        Err(error) => format!("✕ TorrServer: {error}"),
    });
    checks.push(
        match history_path().and_then(|path| {
            HistoryStore::open(&path)
                .map_err(|error| error.to_string())
                .and_then(|history| history.integrity_check())
                .map(|()| path)
        }) {
            Ok(path) => format!("✓ SQLite: {}", path.display()),
            Err(error) => format!("✕ SQLite: {error}"),
        },
    );
    checks.push(match mpv::bundled_executable() {
        Ok(path) if path.is_file() => format!("✓ Встроенный MPV: {}", path.display()),
        Ok(path) => format!("✕ Встроенный MPV: файл не найден — {}", path.display()),
        Err(error) => format!("✕ Встроенный MPV: {error}"),
    });
    checks.push(match settings::validate_player(player_type, player_path) {
        Ok(()) => "✓ Выбранный плеер настроен".into(),
        Err(error) => format!("✕ Выбранный плеер: {error}"),
    });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(6)))
        .build()
        .into();
    checks.push(
        match agent
            .get("https://v3-cinemeta.strem.io/meta/movie/tt0111161.json")
            .call()
        {
            Ok(_) => "✓ Cinemeta: сервис отвечает".into(),
            Err(error) => format!("✕ Cinemeta: {error}"),
        },
    );
    checks
}

#[cfg(windows)]
fn launch_update_installer(installer: &PathBuf) -> Result<(), String> {
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Process -FilePath $env:PIRATE_CINEMA_UPDATE -Verb RunAs",
        ])
        .env("PIRATE_CINEMA_UPDATE", installer)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Не удалось запросить права для установщика: {error}"))
}

#[cfg(not(windows))]
fn launch_update_installer(installer: &PathBuf) -> Result<(), String> {
    std::process::Command::new(installer)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Не удалось запустить установщик: {error}"))
}

fn open_external_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://github.com/cyberboy1999/pirate-cinema/releases/") {
        return Err("Некорректная ссылка обновления".into());
    }
    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Не удалось открыть браузер: {error}"))
}

async fn flush_web_progress() {
    let mut eval = document::eval("dioxus.send(window.__pirateCinemaSnapshot?.() || null);");
    if let Ok(Ok(value)) =
        tokio::time::timeout(Duration::from_millis(700), eval.recv::<serde_json::Value>()).await
    {
        if let (Some(hash), Some(file_id), Some(position), Some(duration)) = (
            value.get("hash").and_then(|value| value.as_str()),
            value.get("file_id").and_then(|value| value.as_i64()),
            value.get("position").and_then(|value| value.as_f64()),
            value.get("duration").and_then(|value| value.as_f64()),
        ) {
            if let Ok(path) = history_path() {
                if let Ok(history) = HistoryStore::open(&path) {
                    let _ = history.save_progress(hash, file_id, position as i64, duration as i64);
                }
            }
        }
    }
}

fn launch_playback(
    request: PendingPlayback,
    endpoint: String,
    mut busy: Signal<bool>,
    mut players: Signal<Vec<OwnedPlayer>>,
    mut web_player: Signal<Option<WebPlayback>>,
    mut server: Signal<ServerState>,
) {
    let window = dioxus::desktop::window();
    if window.window.is_minimized() || !window.window.is_visible() {
        server.write().error = "Восстановите окно приложения, чтобы начать воспроизведение".into();
        return;
    }
    let PendingPlayback {
        torrent,
        file,
        resume,
        queue,
        auto_next,
        force_mpv,
    } = request;
    busy.set(true);
    spawn(async move {
        flush_web_progress().await;
        let player_torrent = torrent.clone();
        let player_file = file.clone();
        let web_queue = queue.clone();
        let result = tokio::task::spawn_blocking(move || {
            let preferences = settings_path()
                .ok()
                .and_then(|path| settings::load_preferences(&path).ok())
                .unwrap_or_else(default_preferences);
            let web_playback = preferences.player_type != PlayerType::External
                && use_web_player(preferences.embedded_player, force_mpv);
            if uses_raw_stream_probe(preferences.player_type, web_playback) {
                probe_stream(&endpoint, &torrent.hash, &file)
                    .map_err(|error| format!("Поток пока недоступен: {error}"))?;
            }
            if preferences.player_type == PlayerType::External {
                if !PLAYBACK_VISIBLE.load(Ordering::Acquire) {
                    return Err("Воспроизведение не запущено: окно свёрнуто".into());
                }
                settings::validate_player(preferences.player_type, &preferences.player_path)?;
                let url = stream_url(&endpoint, &torrent.hash, &file)?;
                std::process::Command::new(preferences.player_path.trim())
                    .arg(url)
                    .spawn()
                    .map_err(|error| format!("Не удалось запустить внешний плеер: {error}"))?;
                let history =
                    HistoryStore::open(&history_path()?).map_err(|error| error.to_string())?;
                history
                    .mark_played(&torrent.hash, file.id, &file.name, Some(&file.path))
                    .map_err(|error| error.to_string())?;
                return Ok(PlaybackLaunch::External);
            }
            if web_playback {
                ensure_gstreamer(&endpoint)?;
                let history =
                    HistoryStore::open(&history_path()?).map_err(|error| error.to_string())?;
                let position = if resume {
                    history
                        .get(&torrent.hash, file.id)
                        .map_err(|error| error.to_string())?
                        .and_then(|record| record.playback_timecode)
                        .unwrap_or(0)
                        .max(0)
                } else {
                    0
                };
                history
                    .mark_played(&torrent.hash, file.id, &file.name, Some(&file.path))
                    .map_err(|error| error.to_string())?;
                let url = gstreamer_hls_url(&endpoint, &torrent.hash, file.id, position);
                let probe_url = format!(
                    "{}/gst/{}/probe?index={}",
                    endpoint.trim_end_matches('/'),
                    torrent.hash,
                    file.id
                );
                let heartbeat_url = format!(
                    "{}/gst/{}/heartbeat",
                    endpoint.trim_end_matches('/'),
                    torrent.hash
                );
                return Ok(PlaybackLaunch::Web(WebPlayback {
                    torrent,
                    file,
                    queue: web_queue,
                    auto_next,
                    url,
                    probe_url,
                    heartbeat_url,
                    position,
                }));
            }
            if !PLAYBACK_VISIBLE.load(Ordering::Acquire) {
                return Err("Воспроизведение не запущено: окно свёрнуто".into());
            }
            MpvSession::launch(
                &mpv::bundled_executable()?,
                &history_path()?,
                &endpoint,
                &torrent.hash,
                &file,
                resume,
            )
            .map(PlaybackLaunch::Mpv)
        })
        .await;
        match result {
            Ok(Ok(PlaybackLaunch::Mpv(session))) => {
                players.write().push(OwnedPlayer {
                    session,
                    torrent: player_torrent,
                    file: player_file,
                    queue,
                    auto_next,
                });
                server.write().error.clear();
            }
            Ok(Ok(PlaybackLaunch::Web(playback))) => {
                web_player.set(Some(playback));
                server.write().error.clear();
            }
            Ok(Ok(PlaybackLaunch::External)) => {
                server.write().error =
                    "Файл открыт во внешнем плеере; позиция просмотра там не отслеживается".into()
            }
            Ok(Err(error)) => server.write().error = error,
            Err(error) => server.write().error = error.to_string(),
        }
        busy.set(false);
    });
}

fn use_web_player(embedded_player: bool, force_mpv: bool) -> bool {
    embedded_player && !force_mpv
}

fn uses_raw_stream_probe(player_type: PlayerType, web_playback: bool) -> bool {
    player_type == PlayerType::External || !web_playback
}

fn gstreamer_hls_url(endpoint: &str, hash: &str, file_id: i64, position: i64) -> String {
    format!(
        "{}/gst/{hash}/master.m3u8?index={}&seconds={}",
        endpoint.trim_end_matches('/'),
        file_id.max(1),
        position.max(0)
    )
}

fn ensure_gstreamer(endpoint: &str) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();
    let mut response = agent
        .get(format!("{}/gst/settings", endpoint.trim_end_matches('/')))
        .call()
        .map_err(|error| format!("TorrServer GST не отвечает: {error}"))?;
    let mut settings: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный ответ TorrServer GST: {error}"))?;
    if settings.get("built_in").and_then(|value| value.as_bool()) != Some(true) {
        return Err("Запущена обычная сборка TorrServer без HLS. Закройте её и перезапустите локальный прототип".into());
    }
    let config = settings
        .get_mut("config")
        .and_then(|value| value.as_object_mut())
        .ok_or("TorrServer GST не вернул конфигурацию")?;
    let mut changed = false;
    for name in [
        "TranscodeH265",
        "TranscodeAV1",
        "TranscodeVP9",
        "TranscodeVP8",
        "TranscodeAVI",
        "HDRToSDR",
        "X264Ultrafast",
        "Subtitles",
    ] {
        if config.get(name).and_then(|value| value.as_bool()) != Some(true) {
            config.insert(name.into(), serde_json::Value::Bool(true));
            changed = true;
        }
    }
    if changed {
        agent
            .post(format!("{}/gst/settings", endpoint.trim_end_matches('/')))
            .send_json(serde_json::json!({"action": "set", "config": config}))
            .map_err(|error| format!("Не удалось настроить HLS-транскодирование: {error}"))?;
    }
    let mut status = agent
        .get(format!("{}/gst/echo", endpoint.trim_end_matches('/')))
        .call()
        .map_err(|error| format!("Не удалось проверить встроенный GStreamer: {error}"))?;
    let status: serde_json::Value = status
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректный ответ GStreamer: {error}"))?;
    if status["gstreamer"]["works"] != true || status["gst_discoverer"]["works"] != true {
        return Err(
            "Встроенный GStreamer TorrServer не работает; проверьте установку приложения".into(),
        );
    }
    Ok(())
}

fn control_player(
    index: usize,
    action: &str,
    mut players: Signal<Vec<OwnedPlayer>>,
    mut server: Signal<ServerState>,
) {
    if action == "stop" {
        if index < players.read().len() {
            players.write().remove(index);
        }
        return;
    }
    let result = players
        .write()
        .get_mut(index)
        .ok_or("Окно MPV уже закрыто".to_owned())
        .and_then(|player| match action {
            "focus" => player.session.focus(),
            "pause" => player.session.toggle_pause(),
            _ => Ok(()),
        });
    if let Err(error) = result {
        server.write().error = error;
    }
}

fn load_library_cards(torrents: &[Torrent]) -> Result<Vec<LibraryCard>, String> {
    let history_path = history_path()?;
    let history = HistoryStore::open(&history_path).map_err(|error| error.to_string())?;
    let poster_dir = history_path
        .parent()
        .ok_or_else(|| "Не удалось определить каталог локального кэша постеров".to_owned())?
        .join("cache")
        .join("posters");
    Ok(torrents
        .iter()
        .cloned()
        .map(|torrent| {
            let metadata = history.metadata(&torrent.hash).ok().flatten();
            let media_type = history.media_type(&torrent.hash).ok().flatten();
            let viewed = history.has_viewed_file(&torrent.hash).unwrap_or(false);
            let poster = metadata
                .as_ref()
                .and_then(|item| item.poster_file.as_ref())
                .and_then(|name| {
                    let path = poster_dir.join(name);
                    let bytes = std::fs::read(path).ok()?;
                    (bytes.len() <= 8 * 1024 * 1024).then(|| {
                        format!(
                            "data:image/jpeg;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        )
                    })
                });
            LibraryCard {
                torrent,
                metadata,
                media_type,
                viewed,
                poster,
            }
        })
        .collect())
}

fn load_continue_items(torrents: &[Torrent]) -> Result<Vec<ContinueItem>, String> {
    let history = HistoryStore::open(&history_path()?).map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    Ok(history
        .recently_played(24)
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|item| {
            if !seen.insert(item.torrent_hash.clone()) {
                return None;
            }
            let torrent = torrents
                .iter()
                .find(|torrent| torrent.hash == item.torrent_hash)?
                .clone();
            let title = history
                .metadata(&torrent.hash)
                .ok()
                .flatten()
                .map_or_else(|| torrent.title.clone(), |metadata| metadata.title);
            Some(ContinueItem {
                torrent,
                title,
                file_name: item.file_name,
                position: item.playback_timecode,
                duration: item.playback_duration,
            })
        })
        .take(12)
        .collect())
}

fn sync_library_metadata(
    endpoint: &str,
    torrents: &[Torrent],
    progress: Option<&AtomicUsize>,
) -> Result<(usize, usize, Vec<LibraryCard>), String> {
    if METADATA_SYNC_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("Обновление карточек уже выполняется".into());
    }
    let _sync_guard = MetadataSyncGuard;
    let path = history_path()?;
    let history = HistoryStore::open(&path).map_err(|error| error.to_string())?;
    let poster_dir = path
        .parent()
        .ok_or("Не удалось определить папку постеров")?
        .join("cache")
        .join("posters");
    std::fs::create_dir_all(&poster_dir).map_err(|error| error.to_string())?;
    let mut updated = 0;
    let mut failed = 0;

    for (index, torrent) in torrents.iter().enumerate() {
        if let Some(progress) = progress {
            progress.store(index, Ordering::Relaxed);
        }
        let current = history
            .metadata(&torrent.hash)
            .map_err(|error| error.to_string())?;
        let titles = metadata_queries(
            current.as_ref().map(|item| item.title.as_str()),
            &torrent.title,
        );
        let stored_type = history
            .media_type(&torrent.hash)
            .map_err(|error| error.to_string())?;
        let series = if let Some(kind) = stored_type.as_deref() {
            kind == "series"
        } else {
            let files = torrent_video_files(endpoint, &torrent.hash).unwrap_or_default();
            let detected = infer_series(&torrent.title, &files);
            history
                .set_media_type(&torrent.hash, if detected { "series" } else { "movie" })
                .map_err(|error| error.to_string())?;
            detected
        };
        let movie = titles
            .iter()
            .find_map(|title| metadata::lookup(title, series).ok().flatten());
        let poster_file = movie.as_ref().and_then(|item| {
            let bytes = metadata::movie_poster_jpeg(item, series).ok()?;
            let name = format!("{}.jpg", torrent.hash);
            std::fs::write(poster_dir.join(&name), bytes).ok()?;
            Some(name)
        });
        let poster_file = poster_file.or_else(|| {
            let files = torrent_video_files(endpoint, &torrent.hash).ok()?;
            let source = if series {
                files.first()
            } else {
                files.iter().max_by_key(|file| file.length)
            }?;
            let executable = mpv::bundled_executable().ok()?;
            let name = format!("{}.jpg", torrent.hash);
            mpv::capture_poster(
                &executable,
                endpoint,
                &torrent.hash,
                source,
                &poster_dir.join(&name),
            )
            .ok()?;
            Some(name)
        });
        let Some(movie) = movie else {
            if let Some(poster_file) = poster_file {
                let metadata = MediaMetadata {
                    title: titles
                        .first()
                        .map(|title| title.split(" / ").next().unwrap_or(title).to_owned())
                        .unwrap_or_else(|| torrent.title.clone()),
                    overview: current.and_then(|item| item.overview),
                    year: None,
                    rating: None,
                    poster_file: Some(poster_file),
                    genres: Vec::new(),
                };
                history
                    .merge_remote_metadata(&torrent.hash, &torrent.title, &metadata)
                    .map_err(|error| error.to_string())?;
                updated += 1;
            } else {
                failed += 1;
            }
            continue;
        };
        let metadata = MediaMetadata {
            title: movie.title,
            overview: movie.overview,
            year: movie.year,
            rating: movie.rating,
            poster_file,
            genres: movie.genres,
        };
        history
            .merge_remote_metadata(&torrent.hash, &torrent.title, &metadata)
            .map_err(|error| error.to_string())?;
        updated += 1;
    }
    if let Some(progress) = progress {
        progress.store(torrents.len(), Ordering::Relaxed);
    }
    let cards = load_library_cards(torrents)?;
    Ok((updated, failed, cards))
}

fn metadata_queries(stored_title: Option<&str>, torrent_title: &str) -> Vec<String> {
    let mut titles = stored_title
        .into_iter()
        .chain(std::iter::once(torrent_title))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .flat_map(metadata::title_candidates)
        .collect::<Vec<_>>();
    titles.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    titles
}

fn missing_metadata_torrents(cards: &[LibraryCard]) -> Vec<Torrent> {
    cards
        .iter()
        .filter(|card| card.metadata.is_none() || card.poster.is_none())
        .map(|card| card.torrent.clone())
        .collect()
}

fn load_file_history(hash: &str, files: Vec<VideoFile>) -> Vec<PlayableFile> {
    let history = history_path()
        .ok()
        .and_then(|path| HistoryStore::open(&path).ok());
    files
        .into_iter()
        .map(|file| {
            let saved = history
                .as_ref()
                .and_then(|store| store.get(hash, file.id).ok().flatten());
            PlayableFile {
                file,
                position: saved
                    .as_ref()
                    .and_then(|item| item.playback_timecode)
                    .unwrap_or(0),
                duration: saved
                    .as_ref()
                    .and_then(|item| item.playback_duration)
                    .unwrap_or(0),
                viewed: saved.is_some_and(|item| item.is_watched),
            }
        })
        .collect()
}

fn clock(seconds: i64) -> String {
    let seconds = seconds.max(0);
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}

fn file_season(path: &str) -> Option<u8> {
    let lower = path.to_lowercase();
    if ["special", "ova", "спецвыпуск"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Some(0);
    }
    for marker in ["season", "сезон"] {
        if let Some(offset) = lower.find(marker) {
            let digits = lower[offset + marker.len()..]
                .trim_start_matches(|character: char| !character.is_ascii_digit())
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            if let Ok(number) = digits.parse::<u8>() {
                return Some(number);
            }
        }
    }
    let bytes = lower.as_bytes();
    for index in 0..bytes.len().saturating_sub(2) {
        if bytes[index] == b's' && bytes[index + 1].is_ascii_digit() {
            let digits = lower[index + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            if let Ok(number) = digits.parse::<u8>() {
                return Some(number);
            }
        }
    }
    None
}

fn file_episode(path: &str) -> Option<u16> {
    let lower = path.to_lowercase();
    for marker in ["episode", "эпизод", "серия"] {
        if let Some(offset) = lower.find(marker) {
            let digits = lower[offset + marker.len()..]
                .trim_start_matches(|character: char| !character.is_ascii_digit())
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            if let Ok(number) = digits.parse::<u16>() {
                return Some(number);
            }
        }
    }
    let bytes = lower.as_bytes();
    for index in 1..bytes.len().saturating_sub(1) {
        if bytes[index] == b'e' && bytes[index - 1].is_ascii_digit() {
            let digits = lower[index + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            if let Ok(number) = digits.parse::<u16>() {
                return Some(number);
            }
        }
    }
    None
}

fn is_episode(path: &str) -> bool {
    file_season(path).is_some() || file_episode(path).is_some()
}

fn next_episode(queue: &[VideoFile], current_id: i64) -> Option<&VideoFile> {
    let position = queue.iter().position(|file| file.id == current_id)?;
    is_episode(&queue[position].path).then_some(())?;
    queue[position + 1..]
        .iter()
        .find(|file| is_episode(&file.path))
}

fn infer_series(title: &str, files: &[VideoFile]) -> bool {
    let episodic = files.iter().filter(|file| is_episode(&file.path)).count();
    episodic > 1
        || episodic * 2 > files.len()
        || is_episode(title)
        || title.to_lowercase().contains("сезон")
}

#[cfg(test)]
mod tests {
    use super::{
        file_episode, file_season, gstreamer_hls_url, history_path, infer_series, metadata_queries,
        missing_metadata_torrents, next_episode, release_quality, short_hash,
        sync_library_metadata, use_web_player, uses_raw_stream_probe, LibraryCard,
    };
    use pirate_cinema_core::{history::HistoryStore, settings::PlayerType, Torrent, VideoFile};

    #[test]
    fn short_hash_never_splits_utf8_or_overflows() {
        assert_eq!(short_hash("ae30a8d74972abcdef"), "ae30a8d74972");
        assert_eq!(short_hash("короткий"), "короткий");
    }

    #[test]
    fn finds_common_season_labels() {
        assert_eq!(file_season("Season_12/Episode 1.mkv"), Some(12));
        assert_eq!(file_season("Сезон 2/серия.mkv"), Some(2));
        assert_eq!(file_season("Show.S03E08.mkv"), Some(3));
        assert_eq!(file_season("Show/OVA 1.mkv"), Some(0));
        assert_eq!(file_season("Movie.2026.mkv"), None);
    }

    #[test]
    fn detects_and_classifies_episodic_releases() {
        assert_eq!(file_episode("Show.S03E08.mkv"), Some(8));
        assert_eq!(file_episode("Сезон 2/Серия 11.mkv"), Some(11));
        let files = ["Movie.mkv", "Show.S01E01.mkv", "Show.S01E02.mkv"]
            .into_iter()
            .enumerate()
            .map(|(id, path)| VideoFile {
                id: id as i64,
                name: path.into(),
                path: path.into(),
                length: 1,
            })
            .collect::<Vec<_>>();
        assert!(infer_series("Collection", &files));
    }

    #[test]
    fn groups_release_quality_like_the_electron_filters() {
        assert_eq!(release_quality("Movie.2160p.UHD.BluRay"), "2160p");
        assert_eq!(release_quality("Movie.1080i.HDTV"), "1080p");
        assert_eq!(release_quality("Movie.720p.WEB-DL"), "720p");
        assert_eq!(release_quality("Movie.DVDRip"), "Другое");
    }

    #[test]
    fn next_episode_skips_movies_and_stops_after_the_last_episode() {
        let files = ["Show.S01E01.mkv", "Film.mkv", "Show.S01E02.mkv"]
            .into_iter()
            .enumerate()
            .map(|(id, path)| VideoFile {
                id: id as i64,
                name: path.into(),
                path: path.into(),
                length: 1,
            })
            .collect::<Vec<_>>();
        assert_eq!(next_episode(&files, 0).map(|file| file.id), Some(2));
        assert!(next_episode(&files, 1).is_none());
        assert!(next_episode(&files, 2).is_none());
    }

    #[test]
    fn mpv_fallback_never_uses_the_web_player() {
        assert!(use_web_player(true, false));
        assert!(!use_web_player(true, true));
        assert!(!use_web_player(false, false));
    }

    #[test]
    fn hls_launch_skips_the_legacy_raw_stream_probe() {
        assert!(!uses_raw_stream_probe(PlayerType::BundledMpv, true));
        assert!(uses_raw_stream_probe(PlayerType::BundledMpv, false));
        assert!(uses_raw_stream_probe(PlayerType::External, false));
    }

    #[test]
    fn gstreamer_hls_url_keeps_file_and_resume_position() {
        assert_eq!(
            gstreamer_hls_url("http://127.0.0.1:8090/", "abc", 7, 42),
            "http://127.0.0.1:8090/gst/abc/master.m3u8?index=7&seconds=42"
        );
    }

    #[test]
    fn refresh_tries_the_saved_title_then_the_original_release_title() {
        assert_eq!(
            metadata_queries(Some("Сёгун"), "Shogun.2024.S01.1080p"),
            ["Сёгун", "Shogun"]
        );
        assert_eq!(metadata_queries(Some("  "), "Тачки"), ["Тачки"]);
    }

    #[test]
    fn startup_refresh_selects_only_incomplete_cards() {
        let complete = LibraryCard {
            torrent: Torrent {
                hash: "a".repeat(40),
                title: "Готово".into(),
            },
            metadata: Some(pirate_cinema_core::history::MediaMetadata {
                title: "Готово".into(),
                overview: Some("Описание".into()),
                year: None,
                rating: None,
                poster_file: Some("a.jpg".into()),
                genres: vec![],
            }),
            media_type: Some("movie".into()),
            viewed: false,
            poster: Some("data:image/jpeg;base64,x".into()),
        };
        let incomplete = LibraryCard {
            torrent: Torrent {
                hash: "b".repeat(40),
                title: "Без постера".into(),
            },
            metadata: complete.metadata.clone(),
            media_type: Some("movie".into()),
            viewed: false,
            poster: None,
        };
        assert_eq!(
            missing_metadata_torrents(&[complete, incomplete])[0].title,
            "Без постера"
        );
    }

    #[test]
    #[ignore = "uses public Cinemeta, TVmaze and Wikidata"]
    fn live_refresh_writes_a_description_and_poster() {
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-metadata-refresh-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        unsafe { std::env::set_var("PIRATE_CINEMA_DATA_DIR", &root) };
        let torrent = Torrent {
            hash: "a".repeat(40),
            title: "Сёгун / Shogun (2024)".into(),
        };
        let history = HistoryStore::open(&history_path().unwrap()).unwrap();
        history.set_media_type(&torrent.hash, "series").unwrap();
        let (updated, failed, cards) =
            sync_library_metadata("http://127.0.0.1:8090", &[torrent], None).unwrap();
        assert_eq!((updated, failed), (1, 0));
        let card = cards.first().unwrap();
        assert!(card
            .metadata
            .as_ref()
            .and_then(|item| item.overview.as_ref())
            .is_some());
        assert!(card.poster.is_some());
        drop(cards);
        drop(history);
        unsafe { std::env::remove_var("PIRATE_CINEMA_DATA_DIR") };
        std::fs::remove_dir_all(root).unwrap();
    }
}
