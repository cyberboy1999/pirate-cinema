use crate::history::HistoryStore;
use crate::{stream_url, VideoFile};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
type IpcStream = std::fs::File;
#[cfg(unix)]
type IpcStream = std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum MpvEvent {
    Loaded,
    Progress { seconds: i64, duration: i64 },
    Ended { eof: bool },
    StreamFailed(String),
    Error(String),
}

pub struct MpvSession {
    child: Child,
    hash: String,
    file_id: i64,
    file_name: String,
    pipe: Option<IpcStream>,
    ipc_path: Option<PathBuf>,
    events: Receiver<MpvEvent>,
    worker: Option<JoinHandle<()>>,
    embedded_host: Option<EmbeddedHost>,
}

struct EmbeddedHost {
    window: isize,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl MpvSession {
    pub fn launch(
        executable: &Path,
        history_path: &Path,
        endpoint: &str,
        hash: &str,
        file: &VideoFile,
        resume: bool,
        embedded: bool,
    ) -> Result<Self, String> {
        if !executable.is_file() {
            return Err(format!(
                "Встроенный MPV не найден: {}",
                executable.display()
            ));
        }
        let history = HistoryStore::open(history_path)
            .map_err(|error| format!("Не удалось открыть историю просмотра: {error}"))?;
        let position = if resume {
            history
                .get(hash, file.id)
                .map_err(|error| error.to_string())?
                .and_then(|record| record.playback_timecode)
                .unwrap_or(0)
                .max(0)
        } else {
            0
        };
        let saved_audio_track = history
            .audio_track(hash)
            .map_err(|error| format!("Не удалось прочитать аудиодорожку: {error}"))?;
        let saved_subtitle_track = history
            .subtitle_track(hash)
            .map_err(|error| format!("Не удалось прочитать дорожку субтитров: {error}"))?;
        let url = stream_url(endpoint, hash, file)?;
        #[cfg(windows)]
        let pipe_name = format!(
            r"\\.\pipe\pirate-cinema-rust-mpv-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_nanos()
        );
        #[cfg(unix)]
        let pipe_name = std::env::temp_dir()
            .join(format!(
                "pirate-cinema-mpv-{}-{}.sock",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| error.to_string())?
                    .as_nanos()
            ))
            .display()
            .to_string();
        let mut command = Command::new(executable);
        command.args([
            "--idle=yes",
            "--force-window=immediate",
            "--keep-open=no",
            "--video-sync=audio",
            "--vo=gpu",
            &format!("--input-ipc-server={pipe_name}"),
        ]);
        #[cfg(windows)]
        let embedded_host = if embedded {
            create_embedded_window().ok()
        } else {
            None
        };
        #[cfg(not(windows))]
        let embedded_host: Option<EmbeddedHost> = None;
        #[cfg(windows)]
        if let Some(host) = embedded_host.as_ref() {
            command.arg(format!("--wid={}", host.window));
        }
        #[cfg(windows)]
        command.args([
            "--gpu-api=opengl",
            "--gpu-context=win",
            "--hwdec=d3d11va-copy",
            "--opengl-swapinterval=1",
            "--priority=high",
        ]);
        #[cfg(unix)]
        command.arg("--hwdec=auto-safe");
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Не удалось запустить MPV: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut pipe = loop {
            match connect_ipc(&pipe_name) {
                Ok(pipe) => break pipe,
                Err(error) => {
                    if child
                        .try_wait()
                        .map_err(|error| error.to_string())?
                        .is_some()
                        || Instant::now() >= deadline
                    {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!("MPV не открыл IPC: {error}"));
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        };
        let reader = match pipe.try_clone() {
            Ok(reader) => reader,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Не удалось открыть IPC MPV: {error}"));
            }
        };
        let options = load_options(file, position, saved_audio_track, saved_subtitle_track);
        for command in [
            json!(["observe_property", 1, "time-pos"]),
            json!(["observe_property", 2, "duration"]),
            json!(["observe_property", 3, "aid"]),
            json!(["observe_property", 4, "sid"]),
            json!(["loadfile", url, "replace", -1, options]),
        ] {
            if let Err(error) = send_command(&mut pipe, command) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Не удалось передать команду MPV: {error}"));
            }
        }
        let (sender, events) = mpsc::channel();
        let session_hash = hash.to_owned();
        let session_file_id = file.id;
        let session_file_name = file.name.clone();
        let hash = hash.to_owned();
        let file = file.clone();
        let worker = thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let mut loaded = false;
            let mut receiving = false;
            let mut seconds = position;
            let mut duration = 0;
            let mut last_saved = Instant::now();
            let save_position = |seconds, duration| match history
                .save_progress(&hash, file.id, seconds, duration)
            {
                Ok(_) => true,
                Err(error) => {
                    let _ = sender.send(MpvEvent::Error(format!(
                        "Не удалось сохранить позицию: {error}"
                    )));
                    false
                }
            };
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) => {
                        let _ =
                            sender.send(MpvEvent::Error(format!("Связь с MPV потеряна: {error}")));
                        break;
                    }
                }
                let Ok(event) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                match event.get("event").and_then(Value::as_str) {
                    Some("start-file") => receiving = true,
                    Some("file-loaded") if receiving && !loaded => {
                        if let Err(error) =
                            history.mark_played(&hash, file.id, &file.name, Some(&file.path))
                        {
                            let _ = sender.send(MpvEvent::Error(format!(
                                "Не удалось сохранить просмотр: {error}"
                            )));
                        } else {
                            loaded = true;
                            let _ = sender.send(MpvEvent::Loaded);
                            save_position(seconds, duration);
                        }
                    }
                    Some("property-change") if receiving => {
                        let value = event.get("data").and_then(Value::as_f64);
                        match event.get("name").and_then(Value::as_str) {
                            Some("time-pos") => {
                                if let Some(value) = value.filter(|value| value.is_finite()) {
                                    seconds = value.max(0.0) as i64;
                                }
                            }
                            Some("duration") => {
                                if let Some(value) = value.filter(|value| value.is_finite()) {
                                    duration = value.max(0.0) as i64;
                                }
                            }
                            Some("aid") => {
                                if let Some(track) = event
                                    .get("data")
                                    .and_then(Value::as_i64)
                                    .filter(|track| *track > 0)
                                {
                                    if let Err(error) = history.save_audio_track(&hash, track) {
                                        let _ = sender.send(MpvEvent::Error(format!(
                                            "Не удалось сохранить аудиодорожку: {error}"
                                        )));
                                    }
                                }
                            }
                            Some("sid") => {
                                if let Some(track) = event
                                    .get("data")
                                    .and_then(Value::as_i64)
                                    .filter(|track| *track > 0)
                                {
                                    if let Err(error) = history.save_subtitle_track(&hash, track) {
                                        let _ = sender.send(MpvEvent::Error(format!(
                                            "Не удалось сохранить субтитры: {error}"
                                        )));
                                    }
                                }
                            }
                            _ => {}
                        }
                        if loaded && last_saved.elapsed() >= Duration::from_secs(5) {
                            if save_position(seconds, duration) {
                                let _ = sender.send(MpvEvent::Progress { seconds, duration });
                            }
                            last_saved = Instant::now();
                        }
                    }
                    Some("end-file") if receiving => {
                        if event.get("reason").and_then(Value::as_str) == Some("error") {
                            let _ = sender.send(MpvEvent::StreamFailed(
                                "MPV не смог открыть поток TorrServer. Проверьте файл через «Диагностику»".into(),
                            ));
                        } else {
                            let eof = event.get("reason").and_then(Value::as_str) == Some("eof");
                            let _ = sender.send(MpvEvent::Ended { eof });
                        }
                        if loaded {
                            save_position(seconds, duration);
                        }
                        receiving = false;
                    }
                    _ => {}
                }
            }
        });
        Ok(Self {
            child,
            hash: session_hash,
            file_id: session_file_id,
            file_name: session_file_name,
            pipe: Some(pipe),
            ipc_path: cfg!(unix).then(|| PathBuf::from(pipe_name)),
            events,
            worker: Some(worker),
            embedded_host,
        })
    }

    pub fn toggle_pause(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("MPV уже закрыт".into());
        }
        let pipe = self.pipe.as_mut().ok_or("IPC MPV закрыт")?;
        send_command(pipe, json!(["cycle", "pause"]))
            .map_err(|error| format!("Не удалось передать команду MPV: {error}"))
    }

    pub fn matches(&self, hash: &str, file_id: i64) -> bool {
        same_media(&self.hash, self.file_id, hash, file_id)
    }

    pub fn label(&self) -> &str {
        &self.file_name
    }

    pub fn is_embedded(&self) -> bool {
        self.embedded_host.is_some()
    }

    pub fn focus(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("MPV уже закрыт".into());
        }
        let pipe = self.pipe.as_mut().ok_or("IPC MPV закрыт")?;
        send_command(pipe, json!(["set_property", "window-minimized", false]))
            .map_err(|error| format!("Не удалось вернуть окно MPV: {error}"))?;
        #[cfg(windows)]
        focus_window_for_process(self.child.id());
        Ok(())
    }

    pub fn try_recv(&self) -> Result<MpvEvent, mpsc::TryRecvError> {
        self.events.try_recv()
    }

    pub fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
}

fn same_media(active_hash: &str, active_file_id: i64, hash: &str, file_id: i64) -> bool {
    active_hash == hash && active_file_id == file_id
}

#[cfg(windows)]
fn focus_window_for_process(pid: u32) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(callback: extern "system" fn(isize, isize) -> i32, value: isize) -> i32;
        fn GetWindowThreadProcessId(window: isize, process: *mut u32) -> u32;
        fn IsWindowVisible(window: isize) -> i32;
        fn ShowWindow(window: isize, command: i32) -> i32;
        fn SetForegroundWindow(window: isize) -> i32;
    }
    extern "system" fn find(window: isize, value: isize) -> i32 {
        let target = unsafe { &mut *(value as *mut (u32, isize)) };
        let mut window_pid = 0;
        unsafe { GetWindowThreadProcessId(window, &mut window_pid) };
        if window_pid == target.0 && unsafe { IsWindowVisible(window) } != 0 {
            target.1 = window;
            0
        } else {
            1
        }
    }
    let mut target = (pid, 0isize);
    unsafe { EnumWindows(find, (&mut target as *mut (u32, isize)) as isize) };
    if target.1 != 0 {
        unsafe {
            ShowWindow(target.1, 9);
            SetForegroundWindow(target.1);
        }
    }
}

impl Drop for MpvSession {
    fn drop(&mut self) {
        self.pipe.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Some(path) = self.ipc_path.take() {
            let _ = std::fs::remove_file(path);
        }
        self.embedded_host.take();
    }
}

#[cfg(windows)]
fn create_embedded_window() -> Result<EmbeddedHost, String> {
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(callback: extern "system" fn(isize, isize) -> i32, value: isize) -> i32;
        fn GetWindowThreadProcessId(window: isize, process: *mut u32) -> u32;
        fn IsWindowVisible(window: isize) -> i32;
        fn GetClientRect(window: isize, rect: *mut Rect) -> i32;
        fn CreateWindowExW(
            extended_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            parameter: isize,
        ) -> isize;
        fn PeekMessageW(
            message: *mut Message,
            window: isize,
            min: u32,
            max: u32,
            remove: u32,
        ) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
        fn DestroyWindow(window: isize) -> i32;
    }
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct Message {
        window: isize,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        point: Point,
        private: u32,
    }
    extern "system" fn find(window: isize, value: isize) -> i32 {
        let target = unsafe { &mut *(value as *mut (u32, isize)) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(window, &mut pid) };
        if pid == target.0 && unsafe { IsWindowVisible(window) } != 0 {
            target.1 = window;
            0
        } else {
            1
        }
    }
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let (stop, stop_receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut target = (std::process::id(), 0isize);
        unsafe { EnumWindows(find, (&mut target as *mut (u32, isize)) as isize) };
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if target.1 == 0 || unsafe { GetClientRect(target.1, &mut rect) } == 0 {
            let _ = ready_sender.send(0);
            return;
        }
        let x = 262;
        let y = 116;
        let width = (rect.right - x - 32).max(640);
        let height = ((width * 9 / 16).min(rect.bottom - y - 120)).max(360);
        let class = "STATIC\0".encode_utf16().collect::<Vec<_>>();
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0x4000_0000 | 0x1000_0000 | 0x0400_0000,
                x,
                y,
                width,
                height,
                target.1,
                0,
                0,
                0,
            )
        };
        let _ = ready_sender.send(window);
        if window == 0 {
            return;
        }
        while stop_receiver.try_recv().is_err() {
            let mut message = Message {
                window: 0,
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                point: Point { x: 0, y: 0 },
                private: 0,
            };
            while unsafe { PeekMessageW(&mut message, 0, 0, 0, 1) } != 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            thread::sleep(Duration::from_millis(16));
        }
        unsafe {
            DestroyWindow(window);
        }
    });
    let window = ready_receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Не удалось создать область встроенного MPV".to_owned())?;
    if window == 0 {
        let _ = worker.join();
        Err("Не удалось создать область встроенного MPV".into())
    } else {
        Ok(EmbeddedHost {
            window,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for EmbeddedHost {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(windows)]
fn connect_ipc(path: &str) -> std::io::Result<IpcStream> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
}

#[cfg(unix)]
fn connect_ipc(path: &str) -> std::io::Result<IpcStream> {
    std::os::unix::net::UnixStream::connect(path)
}

fn send_command(pipe: &mut IpcStream, command: Value) -> std::io::Result<()> {
    writeln!(pipe, "{}", json!({"command": command}))
}

fn load_options(
    file: &VideoFile,
    position: i64,
    audio_track: Option<i64>,
    subtitle_track: Option<i64>,
) -> Value {
    let mut options = json!({"start": position.to_string(), "force-media-title": file.name});
    if let Some(track) = audio_track {
        options["aid"] = json!(track.to_string());
    }
    if let Some(track) = subtitle_track {
        options["sid"] = json!(track.to_string());
    }
    options
}

pub fn bundled_executable() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let beside_app = std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .ok_or("Не удалось определить папку приложения")?
            .join("mpv")
            .join("mpv.exe");
        if beside_app.is_file() {
            return Ok(beside_app);
        }
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("vendor")
            .join("mpv")
            .join("mpv.exe"))
    }
    #[cfg(not(windows))]
    {
        let beside_app = std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .ok_or("Не удалось определить папку приложения")?
            .join("mpv");
        if beside_app.is_file() {
            return Ok(beside_app);
        }
        for path in ["/usr/bin/mpv", "/usr/local/bin/mpv"] {
            if Path::new(path).is_file() {
                return Ok(PathBuf::from(path));
            }
        }
        Err("Системный MPV не найден. Установите пакет mpv".into())
    }
}

pub fn capture_poster(
    executable: &Path,
    endpoint: &str,
    hash: &str,
    file: &VideoFile,
    output: &Path,
) -> Result<(), String> {
    if !executable.is_file() {
        return Err("Встроенный MPV не найден".into());
    }
    if !output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("jpg"))
    {
        return Err("Постер должен иметь формат JPEG".into());
    }
    let directory = output
        .parent()
        .ok_or("Не удалось определить папку постера")?;
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let pattern = directory.join(format!("poster-frame-{stamp}-%01d.jpg"));
    let frame = directory.join(format!("poster-frame-{stamp}-1.jpg"));
    let url = stream_url(endpoint, hash, file)?;
    let mut command = Command::new(executable);
    command
        .args([
            "--no-config",
            "--no-audio",
            "--start=10",
            "--frames=1",
            "--ovc=mjpeg",
            "--vf=scale=600:900:force_original_aspect_ratio=increase,crop=600:900",
            &format!("--o={}", pattern.display()),
            &url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Не удалось запустить MPV для постера: {error}"))?;
    let deadline = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            if !status.success() {
                let _ = std::fs::remove_file(&frame);
                return Err("MPV не смог получить кадр из видеопотока".into());
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&frame);
            return Err("Получение кадра превысило 35 секунд".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let bytes = std::fs::read(&frame).map_err(|_| "MPV не создал кадр".to_owned())?;
    let _ = std::fs::remove_file(&frame);
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("MPV создал повреждённый кадр: {error}"))?;
    if image.width() != 600 || image.height() != 900 {
        return Err("MPV создал кадр неправильного размера".into());
    }
    std::fs::write(output, bytes).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_file_reuses_window_only_for_the_same_torrent_and_index() {
        assert!(same_media("a", 2, "a", 2));
        assert!(!same_media("a", 2, "a", 3));
        assert!(!same_media("a", 2, "b", 2));
    }

    #[test]
    fn command_is_one_json_line() {
        let mut text = Vec::new();
        writeln!(
            &mut text,
            "{}",
            json!({"command": ["observe_property", 1, "time-pos"]})
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&text).unwrap();
        assert_eq!(value["command"][0], "observe_property");
        assert!(text.ends_with(b"\n"));
    }

    #[test]
    fn restores_selected_audio_track_for_next_file() {
        let file = VideoFile {
            id: 7,
            name: "Episode 2.mkv".into(),
            path: "Episode 2.mkv".into(),
            length: 100,
        };
        let options = load_options(&file, 83, Some(2), Some(3));
        assert_eq!(options["start"], "83");
        assert_eq!(options["aid"], "2");
        assert_eq!(options["sid"], "3");
        assert_eq!(options["force-media-title"], "Episode 2.mkv");
        assert!(load_options(&file, 0, None, None).get("aid").is_none());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "starts the bundled MPV without user media"]
    fn bundled_mpv_answers_over_named_pipe() {
        struct OwnedChild(Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let executable = bundled_executable().unwrap();
        let pipe_name = format!(r"\\.\pipe\pirate-cinema-rust-test-{}", std::process::id());
        let _child = OwnedChild(
            Command::new(executable)
                .args([
                    "--no-config",
                    "--idle=yes",
                    "--force-window=no",
                    "--vo=null",
                    "--ao=null",
                    &format!("--input-ipc-server={pipe_name}"),
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut pipe = loop {
            if let Ok(pipe) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&pipe_name)
            {
                break pipe;
            }
            assert!(Instant::now() < deadline, "MPV did not open its IPC pipe");
            thread::sleep(Duration::from_millis(100));
        };
        let reader = pipe.try_clone().unwrap();
        send_command(&mut pipe, json!(["get_property", "mpv-version"])).unwrap();
        send_command(&mut pipe, json!(["cycle", "pause"])).unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut replies = Vec::new();
            for _ in 0..2 {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                replies.push(line);
            }
            sender.send(replies).unwrap();
        });
        let replies = receiver.recv_timeout(Duration::from_secs(3));
        drop(_child);
        let _ = worker.join();
        let replies = replies.unwrap();
        let version: Value = serde_json::from_str(&replies[0]).unwrap();
        let pause: Value = serde_json::from_str(&replies[1]).unwrap();
        assert_eq!(version["error"], "success");
        assert!(version["data"].as_str().unwrap().contains("mpv"));
        assert_eq!(pause["error"], "success");
    }
}
