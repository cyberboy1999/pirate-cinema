//! Local HLS preparation for ArtPlayer. No external player or service is launched.
use crate::{
    desktop_engine::{self, StreamLease},
    VideoFile,
};
use axum::{
    extract::{Path as RoutePath, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, Once,
    },
    time::{Duration, Instant},
};

const SEGMENT: f64 = 3.0;
const WINDOW: usize = 20;
static SERVICE: Mutex<Option<Service>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(1);
static EPOCH: AtomicUsize = AtomicUsize::new(0);
static CACHE_CLEANED: Once = Once::new();
#[derive(Default)]
struct SessionSet {
    items: HashMap<String, Arc<Video>>,
    active: Option<String>,
    pending: Option<String>,
}
type Sessions = Arc<Mutex<SessionSet>>;
struct Service {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    sessions: Sessions,
}
struct Encoder {
    child: Child,
    start: usize,
    end: usize,
    audio: i64,
}
impl Drop for Encoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Video {
    source: String,
    dir: PathBuf,
    probe: Probe,
    encoder: Mutex<Option<Encoder>>,
    closed: AtomicBool,
    _stream: StreamLease,
}
impl Drop for Video {
    fn drop(&mut self) {
        self.stop();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
#[derive(Clone, Debug)]
struct Probe {
    duration: f64,
    tracks: Vec<serde_json::Value>,
}
#[derive(Deserialize, Default)]
struct Options {
    audio: Option<i64>,
}

pub fn executable() -> Result<PathBuf, String> {
    let beside = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Папка приложения недоступна")?
        .join("ffmpeg/ffmpeg.exe");
    if beside.is_file() {
        return Ok(beside);
    }
    if let Some(path) = std::env::var_os("PIRATE_CINEMA_FFMPEG") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    #[cfg(not(windows))]
    for path in ["/usr/bin/ffmpeg", "/usr/local/bin/ffmpeg"] {
        if std::path::Path::new(path).is_file() {
            return Ok(path.into());
        }
    }
    Err("Локальный FFmpeg не найден рядом с приложением".into())
}
fn command() -> Result<Command, String> {
    let mut cmd = Command::new(executable()?);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    Ok(cmd)
}
fn probe(source: &str, epoch: usize) -> Result<Probe, String> {
    let mut cmd = command()?;
    let mut child = cmd
        .args([
            "-hide_banner",
            "-nostdin",
            "-rw_timeout",
            "15000000",
            "-i",
            source,
        ])
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stderr = child.stderr.take().ok_or("Нет вывода FFmpeg")?;
    let reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        let mut source = stderr;
        let mut buf = [0u8; 8192];
        while let Ok(n) = source.read(&mut buf) {
            if n == 0 {
                break;
            }
            if text.len() < 1024 * 1024 {
                text.extend_from_slice(&buf[..n]);
            }
        }
        String::from_utf8_lossy(&text).into_owned()
    });
    let deadline = Instant::now() + Duration::from_secs(35);
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if EPOCH.load(Ordering::Acquire) != epoch {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("Подготовка воспроизведения отменена".into());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err("Не получены параметры видео за 35 секунд".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = reader.join().map_err(|_| "Ошибка чтения FFmpeg")?;
    parse_probe(&output).map_err(|error| {
        let tail = output
            .lines()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" · ");
        if tail.is_empty() {
            error
        } else {
            format!("{error}: {tail}")
        }
    })
}
fn parse_probe(text: &str) -> Result<Probe, String> {
    let time = text
        .split("Duration: ")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .ok_or("FFmpeg не определил длительность")?;
    let parts = time
        .trim()
        .split(':')
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Неизвестная длительность видео")?;
    if parts.len() != 3 {
        return Err("Некорректная длительность".into());
    }
    let duration = parts[0] * 3600.0 + parts[1] * 60.0 + parts[2];
    if !duration.is_finite() || !(0.0..=172800.0).contains(&duration) || duration == 0.0 {
        return Err("Неподдерживаемая длительность видео".into());
    }
    let mut tracks = Vec::new();
    for line in text.lines().filter(|s| s.contains(": Audio:")) {
        let Some(rest) = line.split("Stream #0:").nth(1) else {
            continue;
        };
        let index = rest
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<i64>()
            .map_err(|_| "Некорректная звуковая дорожка")?;
        let lang = rest
            .split('(')
            .nth(1)
            .and_then(|s| s.split(')').next())
            .unwrap_or("");
        tracks.push(serde_json::json!({"Type":"audio","Index":index,"Language":lang,"Title":format!("Дорожка {}",tracks.len()+1)}));
    }
    if !text.contains(": Video:") {
        return Err("В раздаче не найден видеопоток".into());
    }
    Ok(Probe { duration, tracks })
}
fn playlist(duration: f64, audio: i64) -> String {
    let count = (duration / SEGMENT).ceil() as usize;
    let mut text=String::from("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:3\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-INDEPENDENT-SEGMENTS\n");
    for index in 0..count {
        text.push_str(&format!(
            "#EXTINF:{:.6},\nsegment-{index:06}.ts?audio={audio}\n",
            (duration - index as f64 * SEGMENT).min(SEGMENT)
        ));
    }
    text.push_str("#EXT-X-ENDLIST\n");
    text
}
impl Video {
    fn stop(&self) {
        self.closed.store(true, Ordering::Release);
        if let Ok(mut encoder) = self.encoder.lock() {
            encoder.take();
        }
    }
    fn audio(&self, requested: Option<i64>) -> Result<i64, String> {
        let default = self
            .probe
            .tracks
            .first()
            .and_then(|v| v["Index"].as_i64())
            .unwrap_or(-1);
        let audio = requested.unwrap_or(default);
        if audio == -1 && self.probe.tracks.is_empty()
            || self
                .probe
                .tracks
                .iter()
                .any(|v| v["Index"].as_i64() == Some(audio))
        {
            Ok(audio)
        } else {
            Err("Звуковая дорожка не найдена".into())
        }
    }
    fn path(&self, index: usize, audio: i64) -> PathBuf {
        self.dir
            .join(audio.to_string())
            .join(format!("segment-{index:06}.ts"))
    }
    fn ensure(&self, index: usize, audio: i64) -> Result<(), String> {
        if self.closed.load(Ordering::Acquire) {
            return Err("Воспроизведение закрыто".into());
        }
        if index >= (self.probe.duration / SEGMENT).ceil() as usize {
            return Err("Сегмент вне видео".into());
        }
        if self.path(index, audio).is_file() {
            return Ok(());
        }
        let mut encoder = self
            .encoder
            .lock()
            .map_err(|_| "Состояние кодировщика повреждено")?;
        if self.closed.load(Ordering::Acquire) {
            return Err("Воспроизведение закрыто".into());
        }
        if encoder.as_mut().is_some_and(|e| {
            e.audio == audio
                && index >= e.start
                && index < e.end
                && matches!(e.child.try_wait(), Ok(None))
        }) {
            return Ok(());
        }
        encoder.take();
        let dir = self.dir.join(audio.to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let start = index;
        let seconds = start as f64 * SEGMENT;
        let window = (self.probe.duration - seconds).min(WINDOW as f64 * SEGMENT);
        let mut cmd = command()?;
        cmd.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-rw_timeout",
            "15000000",
            "-ss",
            &seconds.to_string(),
            "-i",
            &self.source,
            "-t",
            &window.to_string(),
            "-map",
            "0:v:0",
        ]);
        if audio >= 0 {
            cmd.args([
                "-map",
                &format!("0:{audio}"),
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-ac",
                "2",
            ]);
        } else {
            cmd.arg("-an");
        }
        cmd.args([
            "-vf",
            "scale=trunc(iw/2)*2:trunc(ih/2)*2",
            "-r",
            "24",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "23",
            "-pix_fmt",
            "yuv420p",
            "-g",
            "72",
            "-keyint_min",
            "72",
            "-sc_threshold",
            "0",
            "-force_key_frames",
            "expr:gte(t,n_forced*3)",
            "-output_ts_offset",
            &seconds.to_string(),
            "-f",
            "hls",
            "-hls_time",
            "3",
            "-hls_playlist_type",
            "vod",
            "-hls_flags",
            "temp_file+independent_segments",
            "-start_number",
            &start.to_string(),
            "-hls_segment_filename",
        ])
        .arg(dir.join("segment-%06d.ts"))
        .arg(dir.join("work.m3u8"));
        let child = cmd
            .spawn()
            .map_err(|e| format!("Не удалось подготовить HLS: {e}"))?;
        *encoder = Some(Encoder {
            child,
            start,
            end: start + WINDOW,
            audio,
        });
        Ok(())
    }
}
fn reply(status: StatusCode, kind: &'static str, body: Vec<u8>) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, kind),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}
async fn route(
    axum::extract::State(sessions): axum::extract::State<Sessions>,
    RoutePath((id, asset)): RoutePath<(String, String)>,
    Query(options): Query<Options>,
) -> Response {
    let video = sessions
        .lock()
        .ok()
        .and_then(|sessions| sessions.items.get(&id).cloned());
    let Some(video) = video else {
        return reply(
            StatusCode::NOT_FOUND,
            "text/plain",
            b"Closed playback".to_vec(),
        );
    };
    if asset == "probe" {
        return reply(
            StatusCode::OK,
            "application/json",
            serde_json::to_vec(
                &serde_json::json!({"Tracks":video.probe.tracks,"Duration":video.probe.duration}),
            )
            .unwrap(),
        );
    }
    if asset == "heartbeat" {
        return reply(StatusCode::OK, "application/json", b"{}".to_vec());
    }
    let audio = match video.audio(options.audio) {
        Ok(v) => v,
        Err(e) => return reply(StatusCode::BAD_REQUEST, "text/plain", e.into_bytes()),
    };
    if asset == "master.m3u8" {
        if video.closed.load(Ordering::Acquire) {
            return reply(StatusCode::GONE, "text/plain", b"Closed playback".to_vec());
        }
        let retired = if let Ok(mut sessions) = sessions.lock() {
            if !sessions
                .items
                .get(&id)
                .is_some_and(|current| Arc::ptr_eq(current, &video))
                || video.closed.load(Ordering::Acquire)
            {
                return reply(StatusCode::GONE, "text/plain", b"Closed playback".to_vec());
            }
            if sessions.pending.as_deref() == Some(&id) {
                sessions.active = Some(id.clone());
                sessions.pending = None;
                let stale = sessions
                    .items
                    .keys()
                    .filter(|key| *key != &id)
                    .cloned()
                    .collect::<Vec<_>>();
                stale
                    .into_iter()
                    .filter_map(|key| sessions.items.remove(&key))
                    .collect::<Vec<_>>()
            } else if sessions.active.as_deref() == Some(&id) {
                Vec::new()
            } else {
                return reply(StatusCode::GONE, "text/plain", b"Closed playback".to_vec());
            }
        } else {
            Vec::new()
        };
        for old in retired {
            old.stop();
        }
        return reply(
            StatusCode::OK,
            "application/vnd.apple.mpegurl",
            playlist(video.probe.duration, audio).into_bytes(),
        );
    }
    let index = asset
        .strip_prefix("segment-")
        .and_then(|s| s.strip_suffix(".ts"))
        .and_then(|s| s.parse::<usize>().ok());
    let Some(index) = index else {
        return reply(StatusCode::NOT_FOUND, "text/plain", Vec::new());
    };
    let selected = video.clone();
    match tokio::task::spawn_blocking(move || selected.ensure(index, audio)).await {
        Ok(Ok(())) => {}
        _ => {
            return reply(
                StatusCode::BAD_GATEWAY,
                "text/plain",
                b"HLS preparation failed".to_vec(),
            )
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if video.closed.load(Ordering::Acquire) {
            return reply(StatusCode::GONE, "text/plain", b"Closed playback".to_vec());
        }
        let path = video.path(index, audio);
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 32 * 1024 * 1024 {
                return reply(StatusCode::PAYLOAD_TOO_LARGE, "text/plain", Vec::new());
            }
            if let Ok(bytes) = std::fs::read(path) {
                return reply(StatusCode::OK, "video/mp2t", bytes);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return reply(
                StatusCode::GATEWAY_TIMEOUT,
                "text/plain",
                b"Segment timeout".to_vec(),
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
pub fn epoch() -> usize {
    EPOCH.load(Ordering::Acquire)
}

pub fn prepare(hash: &str, file: &VideoFile) -> Result<(String, String, String), String> {
    prepare_at(hash, file, epoch())
}

pub fn prepare_at(
    hash: &str,
    file: &VideoFile,
    expected_epoch: usize,
) -> Result<(String, String, String), String> {
    let epoch = expected_epoch;
    if EPOCH.load(Ordering::Acquire) != epoch {
        return Err("Подготовка воспроизведения отменена".into());
    }
    CACHE_CLEANED.call_once(|| {
        if let Ok(profile) = desktop_engine::profile_dir() {
            let _ = std::fs::remove_dir_all(profile.join("web-hls"));
        }
    });
    let stream = StreamLease::new(hash)?;
    if EPOCH.load(Ordering::Acquire) != epoch {
        return Err("Подготовка воспроизведения отменена".into());
    }
    let source = desktop_engine::stream_url("", hash, file)?;
    if EPOCH.load(Ordering::Acquire) != epoch {
        return Err("Подготовка воспроизведения отменена".into());
    }
    let probe = probe(&source, epoch)?;
    let dir = desktop_engine::profile_dir()?.join("web-hls").join(format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let video = Arc::new(Video {
        source,
        dir,
        probe,
        encoder: Mutex::new(None),
        closed: AtomicBool::new(false),
        _stream: stream,
    });
    let mut service = SERVICE.lock().map_err(|_| "Состояние HLS повреждено")?;
    if EPOCH.load(Ordering::Acquire) != epoch {
        return Err("Подготовка воспроизведения отменена".into());
    }
    if service.is_none() {
        let sessions = Arc::new(Mutex::new(SessionSet::default()));
        let state = sessions.clone();
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let task = desktop_engine::runtime()?.spawn(async move {
            if let Ok(listener) = tokio::net::TcpListener::from_std(listener) {
                let app = Router::new()
                    .route("/watch/{id}/{asset}", get(route))
                    .with_state(state);
                let _ = axum::serve(listener, app).await;
            }
        });
        *service = Some(Service {
            endpoint: format!("http://127.0.0.1:{port}"),
            task,
            sessions,
        });
    }
    let service = service.as_mut().unwrap();
    let id = format!(
        "{}-{}-{}",
        hash,
        file.id,
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let mut sessions = service
        .sessions
        .lock()
        .map_err(|_| "Состояние HLS повреждено")?;
    let pending = sessions.pending.take();
    let stale_pending = pending
        .filter(|pending| sessions.active.as_ref() != Some(pending))
        .and_then(|pending| sessions.items.remove(&pending));
    if let Some(video) = stale_pending {
        video.stop();
    }
    sessions.pending = Some(id.clone());
    sessions.items.insert(id.clone(), video);
    let base = format!("{}/watch/{id}", service.endpoint);
    Ok((
        format!("{base}/master.m3u8"),
        format!("{base}/probe"),
        format!("{base}/heartbeat"),
    ))
}

pub fn release(manifest_url: &str) {
    let Some(id) = manifest_url
        .split('?')
        .next()
        .and_then(|url| url.strip_suffix("/master.m3u8"))
        .and_then(|url| url.rsplit('/').next())
    else {
        return;
    };
    let video = SERVICE.lock().ok().and_then(|service| {
        let sessions = &service.as_ref()?.sessions;
        let mut sessions = sessions.lock().ok()?;
        if sessions.active.as_deref() == Some(id) {
            sessions.active = None;
        }
        if sessions.pending.as_deref() == Some(id) {
            sessions.pending = None;
        }
        sessions.items.remove(id)
    });
    if let Some(video) = video {
        video.stop();
    }
}

pub fn close() {
    EPOCH.fetch_add(1, Ordering::AcqRel);
    if let Ok(mut service) = SERVICE.lock() {
        if let Some(service) = service.take() {
            service.task.abort();
            if let Ok(mut sessions) = service.sessions.lock() {
                for video in sessions.items.values() {
                    video.stop();
                }
                sessions.items.clear();
                sessions.active = None;
                sessions.pending = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probe_and_vod_seek_timeline() {
        let info=parse_probe("Duration: 00:01:02.50, start: 0\nStream #0:0: Video: h264\nStream #0:2(rus): Audio: aac").unwrap();
        assert_eq!(info.duration, 62.5);
        assert_eq!(info.tracks[0]["Index"], 2);
        let text = playlist(info.duration, 2);
        assert!(text.contains("#EXTINF:2.500000"));
        assert!(text.contains("segment-000020.ts?audio=2"));
        assert!(text.ends_with("#EXT-X-ENDLIST\n"));
        assert!(parse_probe("Duration: N/A").is_err());
    }
}
