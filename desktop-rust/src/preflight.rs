//! Explicit local-only, cancellable selected-file readiness check.
use crate::{desktop_engine, web_stream, VideoFile};
use std::{
    io::Read,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
static CHECK: Mutex<()> = Mutex::new(());
pub fn is_checking() -> bool {
    CHECK.try_lock().is_err()
}
#[derive(Clone, Default)]
pub struct Check {
    pub cancelled: Arc<AtomicBool>,
    pub stage: Arc<Mutex<String>>,
}
impl Check {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    fn update(&self, text: &str) {
        if let Ok(mut stage) = self.stage.lock() {
            *stage = text.into();
        }
    }
}
pub fn run(hash: &str, file: &VideoFile, check: &Check) -> Result<String, String> {
    let started = Instant::now();
    let _exclusive = CHECK
        .try_lock()
        .map_err(|_| "Другая проверка уже выполняется")?;
    if check.cancelled.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    if desktop_engine::is_active(hash)? {
        return Err("Сначала закройте воспроизведение этой раздачи".into());
    }
    web_stream::executable()?;
    check.update("Получение данных выбранного файла…");
    let _lease = desktop_engine::StreamLease::new(hash)?;
    let source = desktop_engine::checked_stream_url(
        hash,
        file,
        &check.cancelled,
        started + Duration::from_secs(90),
    )?;
    if check.cancelled.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    let report = check_source(&source, check, started).map_err(|error| {
        if check.cancelled.load(Ordering::Acquire) {
            error
        } else {
            format!("{error}\n{}", desktop_engine::network_diagnostic(hash))
        }
    })?;
    let stats = desktop_engine::torrent_diagnostics(hash)?;
    let live = &stats["live"]["snapshot"];
    let peers = live["peer_stats"]["seen"].as_u64().unwrap_or(0);
    let bytes = stats["progress_bytes"].as_u64().unwrap_or(0);
    Ok(format!(
        "{report}\nСеть: обнаружено пиров — {peers}; получено и проверено — {} КиБ",
        bytes / 1024
    ))
}
fn check_source(source: &str, check: &Check, started: Instant) -> Result<String, String> {
    let mut command = std::process::Command::new(web_stream::executable()?);
    web_stream::configure_local_transport(&mut command);
    command
        .args([
            "-hide_banner",
            "-nostdin",
            "-rw_timeout",
            "60000000",
            "-i",
            source,
            "-t",
            "3",
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-f",
            "mpegts",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let output = child.stdout.take().ok_or("Нет потока FFmpeg")?;
    let errors = child.stderr.take().ok_or("Нет журнала FFmpeg")?;
    let status = check.clone();
    let log = std::thread::spawn(move || {
        let mut reader = errors;
        let mut all = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if all.len() < 128 * 1024 {
                all.extend_from_slice(&buffer[..n]);
            }
            if String::from_utf8_lossy(&all).contains("Video:") {
                status.update("Видео найдено. Пробная подготовка H.264/AAC…");
            }
        }
        String::from_utf8_lossy(&all).into_owned()
    });
    let bytes = std::thread::spawn(move || {
        let mut input = output;
        let mut count = 0u64;
        let mut buffer = [0; 8192];
        while let Ok(n) = input.read(&mut buffer) {
            if n == 0 {
                break;
            }
            count += n as u64;
        }
        count
    });
    let result = loop {
        if check.cancelled.load(Ordering::Acquire) {
            break Err("Проверка отменена".to_string());
        }
        if started.elapsed() > Duration::from_secs(90) {
            break Err("Пока не удалось проверить: данные или обработка не уложились в 90 секунд. Это не означает, что раздача недоступна.".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status.success()),
            Ok(None) => {}
            Err(e) => break Err(e.to_string()),
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let _ = child.kill();
    let _ = child.wait();
    let text = log.join().map_err(|_| "Ошибка чтения диагностики")?;
    let count = bytes.join().map_err(|_| "Ошибка пробного потока")?;
    let succeeded = result?;
    if !succeeded || count < 188 {
        return Err("Пока не удалось подготовить пробный фрагмент. Возможны нехватка пиров, медленная загрузка или неподдерживаемый файл. Попробуйте повторить проверку или выбрать другую раздачу.".into());
    }
    Ok(summarize(&text, started.elapsed().as_secs(), count))
}
fn summarize(text: &str, seconds: u64, bytes: u64) -> String {
    let mut lines = vec![
        format!("Готова к просмотру · проверено за {seconds} с"),
        format!("Подготовлен пробный поток H.264/AAC: {} КиБ", bytes / 1024),
    ];
    for line in text.lines().take_while(|l| !l.starts_with("Output #")) {
        if line.contains("Duration:")
            || line.contains("Video:")
            || line.contains("Audio:")
            || line.contains("Subtitle:")
        {
            lines.push(line.trim().to_owned());
        }
    }
    if text.contains("bt2020") || text.contains("smpte2084") || text.contains("arib-std-b67") {
        lines.push("Обнаружены признаки HDR. Пробная обработка не подтверждает корректность цветов HDR на вашем экране.".into());
    }
    lines.push("Проверен короткий фрагмент в начале файла. Это не гарантирует доступность всей раздачи. История просмотра не изменена.".into());
    lines.join("\n")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_uses_input_tracks_only() {
        let s=summarize("Duration: 00:20:00\nStream #0:0: Video: hevc\nStream #0:1(rus): Audio: ac3\nOutput #0\nStream #0:0: Video: h264",4,1880);
        assert!(s.contains("hevc"));
        assert!(s.contains("ac3"));
        assert!(!s.contains("Video: h264"));
    }
    #[test]
    fn cancellation_is_shared() {
        let a = Check::default();
        let b = a.clone();
        a.cancel();
        assert!(b.cancelled.load(Ordering::Acquire));
    }
    #[test]
    #[ignore = "requires an explicit owned video fixture and local FFmpeg"]
    fn owned_video_probe() {
        let source = std::env::var("PIRATE_CINEMA_PREFLIGHT_FIXTURE").unwrap();
        let result = check_source(&source, &Check::default(), Instant::now()).unwrap();
        assert!(result.contains("Готова к просмотру"));
        assert!(result.contains("Video:"));
    }
}
