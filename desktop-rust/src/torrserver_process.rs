use crate::DEFAULT_TORRSERVER_URL;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct TorrServerProcess {
    child: Option<Child>,
}

impl TorrServerProcess {
    pub fn connect_or_start(
        endpoint: &str,
        executable: &Path,
        data_dir: &Path,
    ) -> Result<Self, String> {
        if probe(endpoint) {
            return Ok(Self { child: None });
        }
        if endpoint.trim_end_matches('/') != DEFAULT_TORRSERVER_URL {
            return Err(
                "Настроенный TorrServer не отвечает; автозапуск доступен только для 127.0.0.1:8090"
                    .into(),
            );
        }
        if TcpListener::bind(("127.0.0.1", 8090)).is_err() {
            return Err(
                "Порт 8090 занят, но TorrServer не отвечает. Проверьте запущенный сервис".into(),
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
        let mut command = launch_command(executable, data_dir);
        let mut child = command
            .spawn()
            .map_err(|error| format!("Не удалось запустить TorrServer: {error}"))?;
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            if probe(endpoint) {
                // RuTor is optional: a transient settings error must not take down
                // an otherwise healthy local TorrServer.
                let _ = enable_rutor_search(endpoint, data_dir);
                return Ok(Self { child: Some(child) });
            }
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                return Err(format!("TorrServer завершился при запуске: {status}"));
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        let _ = child.kill();
        let _ = child.wait();
        Err("TorrServer не ответил за 60 секунд".into())
    }

    pub fn owns_process(&self) -> bool {
        self.child.is_some()
    }
}

fn launch_command(executable: &Path, data_dir: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["--ip", "127.0.0.1", "--port", "8090", "--path"])
        .arg(data_dir)
        .current_dir(data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
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
        let name = "TorrServer-windows-amd64.exe";
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
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA не задан")?;
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or("HOME не задан")?;
    Ok(PathBuf::from(root).join("Pirate Cinema").join("torrserver"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

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
                "8090",
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
        drop(process);
        server.join().unwrap();
    }

    #[test]
    #[ignore = "starts the bundled TorrServer on port 8090"]
    fn starts_and_stops_bundled_torrserver() {
        assert!(
            !probe(DEFAULT_TORRSERVER_URL),
            "port 8090 is already in use"
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
        let settings: serde_json::Value = ureq::post("http://127.0.0.1:8090/settings")
            .send_json(serde_json::json!({"action": "get"}))
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert_eq!(settings["EnableRutorSearch"], true);
        drop(process);
    }
}
