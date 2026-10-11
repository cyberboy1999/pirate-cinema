use crate::DEFAULT_TORRSERVER_URL;
use std::collections::VecDeque;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct LaunchLog {
    executable: String,
    directory: String,
    started: u64,
    pid: Option<u32>,
    peer_port: Option<u16>,
    status: String,
    output: VecDeque<String>,
}
static LAST_LAUNCH: Mutex<Option<Arc<Mutex<LaunchLog>>>> = Mutex::new(None);

fn capture_output(
    mut stream: impl Read + Send + 'static,
    log: Arc<Mutex<LaunchLog>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut bytes = [0u8; 2048];
        while let Ok(size) = stream.read(&mut bytes) {
            if size == 0 {
                break;
            }
            if let Ok(mut log) = log.lock() {
                if log.output.len() == 32 {
                    log.output.pop_front();
                }
                log.output
                    .push_back(String::from_utf8_lossy(&bytes[..size]).into_owned());
            }
        }
    })
}

/// Raw output is local-only: never include it in a shareable report.
pub fn launch_diagnostics() -> (Vec<String>, String) {
    let log = LAST_LAUNCH.lock().ok().and_then(|log| log.clone());
    let Some(log) = log else {
        return (
            vec!["Запуск из приложения ещё не выполнялся".into()],
            "Launch: not attempted".into(),
        );
    };
    let Ok(log) = log.lock() else {
        return (vec![], "Launch: unavailable".into());
    };
    format_launch_log(&log)
}

fn format_launch_log(log: &LaunchLog) -> (Vec<String>, String) {
    let local = vec![
        format!("EXE: {}", log.executable),
        format!("Рабочая папка: {}", log.directory),
        format!(
            "Аргументы: --ip 127.0.0.1 --port 8091 --path <рабочая папка>{}",
            log.peer_port
                .map(|port| format!(" --torrentaddr :{port}"))
                .unwrap_or_default()
        ),
        format!(
            "Запуск (Unix UTC): {} · PID: {:?} · {}",
            log.started, log.pid, log.status
        ),
        format!(
            "Последний вывод (только локально):\n{}",
            log.output.iter().cloned().collect::<String>()
        ),
    ];
    (
        local,
        format!(
            "Launch: {}\nPID: {:?}\nStarted UTC epoch: {}",
            log.status, log.pid, log.started
        ),
    )
}

pub struct TorrServerProcess {
    child: Option<Child>,
    log: Option<Arc<Mutex<LaunchLog>>>,
}

impl TorrServerProcess {
    pub fn connect_or_start(
        endpoint: &str,
        executable: &Path,
        data_dir: &Path,
    ) -> Result<Self, String> {
        Self::connect_with_peer_port(endpoint, executable, data_dir, None)
    }

    fn connect_with_peer_port(
        endpoint: &str,
        executable: &Path,
        data_dir: &Path,
        mut peer_port: Option<u16>,
    ) -> Result<Self, String> {
        if probe(endpoint) {
            return Ok(Self {
                child: None,
                log: None,
            });
        }
        let log = Arc::new(Mutex::new(LaunchLog {
            executable: executable.display().to_string(),
            directory: data_dir.display().to_string(),
            started: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            status: "preflight".into(),
            peer_port,
            ..Default::default()
        }));
        if let Ok(mut latest) = LAST_LAUNCH.lock() {
            *latest = Some(log.clone());
        }
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut result;
        for attempt in 0..3 {
            result = Self::start_checked(
                endpoint,
                executable,
                data_dir,
                log.clone(),
                peer_port,
                deadline,
            );
            if result.is_ok() || !cfg!(windows) || attempt == 2 || Instant::now() >= deadline {
                return Self::record_result(result, &log);
            }
            let retry = log.lock().is_ok_and(|log| {
                retry_random_peer_port(
                    &log.output.iter().cloned().collect::<String>(),
                    peer_port.is_some(),
                )
            });
            if !retry {
                return Self::record_result(result, &log);
            }
            peer_port = match available_peer_port() {
                Ok(port) => Some(port),
                Err(_) => return Self::record_result(result, &log),
            };
            if let Ok(mut log) = log.lock() {
                log.output.clear();
                log.peer_port = peer_port;
                log.status = "preflight".into();
            }
        }
        unreachable!("bounded attempts always return")
    }

    fn record_result(
        result: Result<Self, String>,
        log: &Arc<Mutex<LaunchLog>>,
    ) -> Result<Self, String> {
        if let Err(error) = &result {
            if let Ok(mut log) = log.lock() {
                if log.status == "preflight" {
                    log.status = "preflight failed".into();
                }
                if log.output.len() == 32 {
                    log.output.pop_front();
                }
                log.output.push_back(error.chars().take(2048).collect());
            }
        }
        result
    }

    fn start_checked(
        endpoint: &str,
        executable: &Path,
        data_dir: &Path,
        log: Arc<Mutex<LaunchLog>>,
        peer_port: Option<u16>,
        deadline: Instant,
    ) -> Result<Self, String> {
        if endpoint.trim_end_matches('/') != DEFAULT_TORRSERVER_URL {
            return Err(
                "Настроенный TorrServer не отвечает; автозапуск доступен только для 127.0.0.1:8091"
                    .into(),
            );
        }
        if TcpListener::bind(("127.0.0.1", 8091)).is_err() {
            return Err(
                "Порт 8091 занят, но TorrServer не отвечает. Проверьте запущенный сервис".into(),
            );
        }
        if !executable.is_file() {
            return Err(format!(
                "Файл TorrServer не найден: {}",
                executable.display()
            ));
        }
        std::fs::create_dir_all(data_dir)
            .map_err(|error| format!("Не удалось создать каталог TorrServer: {error}"))?;
        let database = data_dir.join("config.db");
        if database.exists() {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&database)
                .map_err(|error| {
                    format!(
                        "Нет доступа к базе TorrServer {}: {error}",
                        database.display()
                    )
                })?;
        }
        if let Ok(mut log) = log.lock() {
            log.status = "starting".into();
        }
        let mut command = launch_command(executable, data_dir);
        if let Some(port) = peer_port {
            command.args(["--torrentaddr", &format!(":{port}")]);
        }
        let mut child = command.spawn().map_err(|error| {
            if let Ok(mut log) = log.lock() {
                log.status = format!("spawn failed, OS code {:?}", error.raw_os_error());
            }
            format!("Не удалось запустить TorrServer: {error}")
        })?;
        if let Ok(mut log) = log.lock() {
            log.pid = Some(child.id());
        }
        let readers = [
            child
                .stdout
                .take()
                .map(|stream| capture_output(stream, log.clone())),
            child
                .stderr
                .take()
                .map(|stream| capture_output(stream, log.clone())),
        ];
        while Instant::now() < deadline {
            if probe(endpoint) {
                // RuTor is optional: a transient settings error must not take down
                // an otherwise healthy local TorrServer.
                let _ = enable_rutor_search(endpoint, data_dir);
                if let Ok(mut log) = log.lock() {
                    log.status = "running".into();
                }
                return Ok(Self {
                    child: Some(child),
                    log: Some(log),
                });
            }
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                for reader in readers.into_iter().flatten() {
                    let _ = reader.join();
                }
                if let Ok(mut log) = log.lock() {
                    log.status = format!("exit: {status}");
                }
                return Err(format!("TorrServer завершился при запуске: {status}"));
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        let _ = child.kill();
        let _ = child.wait();
        if let Ok(mut log) = log.lock() {
            log.status = "startup timeout (60s), child stopped".into();
        }
        Err("TorrServer не ответил за 60 секунд".into())
    }

    pub fn owns_process(&self) -> bool {
        self.child.is_some()
    }

    pub fn diagnostic_pid(&mut self) -> Option<u32> {
        let child = self.child.as_mut()?;
        match child.try_wait() {
            Ok(Some(status)) => {
                if let Some(log) = &self.log {
                    if let Ok(mut log) = log.lock() {
                        log.status = format!("exit: {status}");
                    }
                }
                None
            }
            Ok(None) => Some(child.id()),
            Err(_) => None,
        }
    }

    pub fn restart_or_start(
        process: &mut Option<Self>,
        endpoint: &str,
        executable: &Path,
        data_dir: &Path,
    ) -> Result<(), String> {
        if endpoint.trim_end_matches('/') != DEFAULT_TORRSERVER_URL {
            return Err(
                "Перезапуск доступен только для локального сервера приложения на 8091".into(),
            );
        }
        if process
            .as_ref()
            .is_some_and(|process| !process.owns_process())
            || (process.is_none() && probe(endpoint))
        {
            return Err("Этот TorrServer запущен вне приложения; перезапустите его вручную".into());
        }
        drop(process.take());
        *process = Some(Self::connect_or_start(endpoint, executable, data_dir)?);
        Ok(())
    }
}

// TorrServer chooses random peer ports itself; Windows can reserve that number.
// Ask the OS for a bindable UDP/TCP pair instead. Never change saved settings.
fn available_peer_port() -> Result<u16, std::io::Error> {
    for _ in 0..16 {
        let udp = std::net::UdpSocket::bind(("0.0.0.0", 0))?;
        let port = udp.local_addr()?.port();
        if let Ok(_tcp) = TcpListener::bind(("0.0.0.0", port)) {
            return Ok(port);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        "No available peer port",
    ))
}

fn retry_random_peer_port(output: &str, selected_by_app: bool) -> bool {
    (selected_by_app || output.contains("Set listen port to random autoselect (0)"))
        && output.contains("BTS.Connect() error!")
        && output.contains("subsequent listen:")
        && output.contains("bind:")
        && output.contains("forbidden by its access permissions")
}

pub fn health_version(endpoint: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let version = agent
        .get(format!("{}/echo", endpoint.trim_end_matches('/')))
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    if version.trim().is_empty() || version.len() > 128 {
        return Err("Некорректный ответ TorrServer /echo".into());
    }
    Ok(version.trim().to_string())
}

fn launch_command(executable: &Path, data_dir: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["--ip", "127.0.0.1", "--port", "8091", "--path"])
        .arg(data_dir)
        .current_dir(data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn enable_rutor_search(endpoint: &str, data_dir: &Path) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(6)))
        .build()
        .into();
    let url = format!("{}/settings", endpoint.trim_end_matches('/'));
    let mut settings: serde_json::Value = agent
        .post(&url)
        .send_json(serde_json::json!({"action": "get"}))
        .map_err(|error| format!("Не удалось прочитать настройки TorrServer: {error}"))?
        .body_mut()
        .read_json()
        .map_err(|error| format!("Некорректные настройки TorrServer: {error}"))?;
    let fields = settings
        .as_object_mut()
        .ok_or("TorrServer вернул некорректные настройки")?;
    fields.insert("EnableRutorSearch".into(), true.into());
    let result = agent
        .post(&url)
        .send_json(serde_json::json!({"action": "set", "sets": settings}))
        .map_err(|error| format!("Не удалось включить поиск RuTor: {error}"));
    if let Err(error) = result {
        let saved = std::fs::read_to_string(data_dir.join("settings.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        if saved
            .as_ref()
            .and_then(|value| value.get("BitTorr"))
            .and_then(|value| value.get("EnableRutorSearch"))
            != Some(&serde_json::Value::Bool(true))
        {
            return Err(error);
        }
    }
    Ok(())
}

impl Drop for TorrServerProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(log) = &self.log {
                if let Ok(mut log) = log.lock() {
                    log.status = "stopped by application".into();
                }
            }
        }
    }
}

fn probe(endpoint: &str) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(800)))
        .build()
        .into();
    agent
        .get(format!("{}/echo", endpoint.trim_end_matches('/')))
        .call()
        .is_ok()
}

pub fn bundled_executable() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let app_dir = std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .ok_or("Не удалось определить папку приложения")?
            .join("torrserver");
        let executable = app_dir.join("TorrServer-gst-windows-amd64.exe");
        if executable.is_file() {
            return Ok(executable);
        }
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("vendor")
            .join("torrserver");
        let gst = vendor.join("TorrServer-gst-windows-amd64.exe");
        Ok(gst)
    }
    #[cfg(not(windows))]
    {
        let name = "TorrServer-linux-amd64";
        let beside_app = std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .ok_or("Не удалось определить папку приложения")?
            .join("torrserver")
            .join(name);
        if beside_app.is_file() {
            return Ok(beside_app);
        }
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("vendor")
            .join("torrserver")
            .join(name))
    }
}

pub fn default_data_dir() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("PIRATE_CINEMA_DATA_DIR") {
        return Ok(PathBuf::from(root).join("torrserver"));
    }
    if let Some(app_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let portable = app_dir.join("portable-data");
        if portable.is_dir() {
            return Ok(portable.join("torrserver"));
        }
    }
    #[cfg(windows)]
    let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA не задан")?);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or("HOME не задан")?;
    Ok(root.join("Pirate Cinema").join("torrserver"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn retries_only_random_windows_peer_bind_denials() {
        let log = "Set listen port to random autoselect (0)\nBTS.Connect() error! subsequent listen: listen udp4 :54463: bind: An attempt was made to access a socket in a way forbidden by its access permissions.";
        assert!(retry_random_peer_port(log, false));
        assert!(!retry_random_peer_port(
            &log.replace("random autoselect (0)", "fixed (54463)"),
            false
        ));
        assert!(!retry_random_peer_port(
            &log.replace(
                "forbidden by its access permissions",
                "address already in use"
            ),
            false
        ));
        assert!(!retry_random_peer_port("database access denied", true));
    }

    #[test]
    fn peer_port_is_selected_by_binding_both_protocols() {
        let port = available_peer_port().unwrap();
        assert_ne!(port, 0);
        let _udp = std::net::UdpSocket::bind(("0.0.0.0", port)).unwrap();
        let _tcp = TcpListener::bind(("0.0.0.0", port)).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "starts GST using an excluded UDP port, then verifies automatic recovery"]
    fn recovers_from_windows_excluded_peer_port() {
        let blocked: u16 = std::env::var("PIRATE_TEST_BLOCKED_PEER_PORT")
            .expect("set a Windows excluded UDP port")
            .parse()
            .unwrap();
        assert_eq!(
            std::net::UdpSocket::bind(("0.0.0.0", blocked))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(
            !probe(DEFAULT_TORRSERVER_URL),
            "port 8091 is already in use"
        );
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("torrserver-peer-recovery-{}", std::process::id()));
        let result = TorrServerProcess::connect_with_peer_port(
            DEFAULT_TORRSERVER_URL,
            &bundled_executable().unwrap(),
            &root,
            Some(blocked),
        );
        if let Err(error) = &result {
            let output = LAST_LAUNCH
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .output
                .iter()
                .cloned()
                .collect::<String>();
            panic!(
                "{error}\n{}",
                output
                    .lines()
                    .filter(|line| line.contains("listen")
                        || line.contains("BTS.Connect")
                        || line.contains("error!"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        let process = result.unwrap();
        assert!(process.owns_process());
        let port = process
            .log
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .peer_port
            .unwrap();
        assert_ne!(port, blocked);
        assert!(health_version(DEFAULT_TORRSERVER_URL)
            .unwrap()
            .contains("MatriX"));
        drop(process);
        assert!(!probe(DEFAULT_TORRSERVER_URL));
    }

    #[test]
    fn launch_log_is_bounded_and_export_excludes_private_output() {
        let log = Arc::new(Mutex::new(LaunchLog {
            executable: "C:/Users/private/secret.exe".into(),
            directory: "private-profile".into(),
            status: "exit: 1".into(),
            ..Default::default()
        }));
        capture_output(std::io::Cursor::new(vec![b'x'; 100_000]), log.clone())
            .join()
            .unwrap();
        let mut log = log.lock().unwrap();
        assert_eq!(log.output.len(), 32);
        log.output
            .push_back("apikey=private-token magnet:?xt=secret torrent-title".into());
        let (local, report) = format_launch_log(&log);
        assert!(local.join("\n").contains("private-token"));
        for private in ["private", "apikey", "magnet", "torrent-title", "secret.exe"] {
            assert!(!report.contains(private));
        }
        assert!(report.contains("exit: 1"));
    }

    #[test]
    fn bundled_server_receives_explicit_network_and_data_arguments() {
        let command = launch_command(Path::new("TorrServer"), Path::new("profile/torrserver"));
        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                "--ip",
                "127.0.0.1",
                "--port",
                "8091",
                "--path",
                "profile/torrserver"
            ]
        );
    }

    #[test]
    fn reuses_running_server_without_starting_or_stopping_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1024];
            let size = stream.read(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request[..size]).contains("GET /echo"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
                .unwrap();
        });
        let process = TorrServerProcess::connect_or_start(
            &format!("http://{address}"),
            Path::new("missing-server.exe"),
            Path::new("unused-data"),
        )
        .unwrap();
        assert!(!process.owns_process());
        let mut saved = Some(process);
        assert!(TorrServerProcess::restart_or_start(
            &mut saved,
            DEFAULT_TORRSERVER_URL,
            Path::new("missing-server.exe"),
            Path::new("unused-data")
        )
        .unwrap_err()
        .contains("вне приложения"));
        let process = saved.take().unwrap();
        drop(process);
        server.join().unwrap();
    }

    #[test]
    #[ignore = "starts the bundled TorrServer on port 8091"]
    fn starts_and_stops_bundled_torrserver() {
        assert!(
            !probe(DEFAULT_TORRSERVER_URL),
            "port 8091 is already in use"
        );
        let executable = bundled_executable().unwrap();
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("torrserver-smoke-data-v2");
        let process =
            TorrServerProcess::connect_or_start(DEFAULT_TORRSERVER_URL, &executable, &data_dir)
                .unwrap();
        assert!(process.owns_process());
        assert!(probe(DEFAULT_TORRSERVER_URL));
        let previous_pid = process.child.as_ref().unwrap().id();
        let mut process = Some(process);
        TorrServerProcess::restart_or_start(
            &mut process,
            DEFAULT_TORRSERVER_URL,
            &executable,
            &data_dir,
        )
        .unwrap();
        assert_ne!(
            process.as_ref().unwrap().child.as_ref().unwrap().id(),
            previous_pid
        );
        assert!(probe(DEFAULT_TORRSERVER_URL));
        let settings: serde_json::Value = ureq::post("http://127.0.0.1:8091/settings")
            .send_json(serde_json::json!({"action": "get"}))
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert_eq!(settings["EnableRutorSearch"], true);
        drop(process);
    }
}
