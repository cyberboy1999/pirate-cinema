//! Opt-in, token-protected LAN controls for the active ArtPlayer session.

use base64::Engine as _;
use qrcode::{Color, QrCode};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

static SERVICE: Mutex<Option<Service>> = Mutex::new(None);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteInfo {
    pub url: String,
    pub qr_data_uri: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RemoteCommand {
    Toggle,
    SeekRelative { seconds: f64 },
    SeekAbsolute { seconds: f64 },
    Volume { value: f64 },
    Next,
    Fullscreen,
    Audio { value: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteStatus {
    pub available: bool,
    pub title: String,
    pub playing: bool,
    pub current_time: f64,
    pub duration: f64,
    pub volume: f64,
    pub audio: Vec<RemoteAudio>,
    pub selected_audio: String,
    pub fullscreen: bool,
}

impl Default for RemoteStatus {
    fn default() -> Self {
        Self {
            available: false,
            title: String::new(),
            playing: false,
            current_time: 0.0,
            duration: 0.0,
            volume: 1.0,
            audio: Vec::new(),
            selected_audio: String::new(),
            fullscreen: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RemoteAudio {
    pub value: String,
    pub label: String,
}

struct Shared {
    commands: Mutex<Vec<RemoteCommand>>,
    status: Mutex<RemoteStatus>,
}

struct Service {
    info: RemoteInfo,
    stop: Arc<AtomicBool>,
    shared: Arc<Shared>,
    active: Arc<Mutex<Option<TcpStream>>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut active) = self.active.lock() {
            if let Some(stream) = active.take() {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start() -> Result<RemoteInfo, String> {
    let mut service = SERVICE.lock().map_err(|_| "Пульт занят")?;
    if let Some(current) = service.as_ref() {
        return Ok(current.info.clone());
    }
    #[cfg(not(test))]
    let bind_address = ("0.0.0.0", 8787);
    #[cfg(test)]
    let bind_address = ("127.0.0.1", 0);
    let listener = TcpListener::bind(bind_address)
        .map_err(|error| format!("Не удалось открыть локальный порт: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let token = session_token()?;
    #[cfg(not(test))]
    let host = lan_ip();
    #[cfg(test)]
    let host = "127.0.0.1".to_owned();
    let url = format!("http://{host}:{port}/{token}/");
    let info = RemoteInfo {
        qr_data_uri: qr_data_uri(&url)?,
        url,
    };
    let stop = Arc::new(AtomicBool::new(false));
    let shared = Arc::new(Shared {
        commands: Mutex::new(Vec::new()),
        status: Mutex::new(RemoteStatus::default()),
    });
    let task_stop = stop.clone();
    let task_shared = shared.clone();
    let task_token = token.clone();
    let active = Arc::new(Mutex::new(None));
    let task_active = active.clone();
    let thread = std::thread::Builder::new()
        .name("pirate-cinema-remote".into())
        .spawn(move || {
            while !task_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Ok(mut active) = task_active.lock() {
                            *active = stream.try_clone().ok();
                        }
                        if !task_stop.load(Ordering::Acquire) {
                            handle(stream, &task_token, &task_shared);
                        }
                        if let Ok(mut active) = task_active.lock() {
                            active.take();
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(40));
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|error| error.to_string())?;
    *service = Some(Service {
        info: info.clone(),
        stop,
        shared,
        active,
        thread: Some(thread),
    });
    Ok(info)
}

pub fn stop() {
    if let Ok(mut service) = SERVICE.lock() {
        service.take();
    }
}

pub fn info() -> Option<RemoteInfo> {
    SERVICE.lock().ok()?.as_ref().map(|item| item.info.clone())
}

pub fn take_commands() -> Vec<RemoteCommand> {
    let Ok(service) = SERVICE.lock() else {
        return Vec::new();
    };
    let Some(service) = service.as_ref() else {
        return Vec::new();
    };
    let Ok(mut commands) = service.shared.commands.lock() else {
        return Vec::new();
    };
    std::mem::take(&mut *commands)
}

pub fn update_status(status: RemoteStatus) {
    if let Ok(service) = SERVICE.lock() {
        if let Some(service) = service.as_ref() {
            if let Ok(mut current) = service.shared.status.lock() {
                *current = status;
            }
        }
    }
}

fn random_token() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| format!("Нет системного генератора: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
fn session_token() -> Result<String, String> {
    random_token()
}

#[cfg(not(test))]
fn session_token() -> Result<String, String> {
    let path = crate::desktop_engine::profile_dir()?.join("remote-token");
    if let Ok(token) = std::fs::read_to_string(&path) {
        let token = token.trim();
        if token.len() == 32 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            secure_token_permissions(&path)?;
            return Ok(token.to_ascii_lowercase());
        }
    }
    let token = random_token()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if path.exists() {
        std::fs::remove_file(&path).map_err(|error| error.to_string())?;
    }
    write_token(&path, &token)?;
    Ok(token)
}

#[cfg(unix)]
fn secure_token_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| error.to_string())
}

#[cfg(all(not(test), not(unix)))]
fn secure_token_permissions(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}

#[cfg(any(not(test), unix))]
fn write_token(path: &std::path::Path, token: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    let temporary = path.with_extension(format!("{}.tmp", random_token()?));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    file.write_all(token.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        error.to_string()
    })?;
    secure_token_permissions(path)?;
    Ok(())
}

fn qr_data_uri(url: &str) -> Result<String, String> {
    let code = QrCode::new(url.as_bytes()).map_err(|error| error.to_string())?;
    let width = code.width();
    let mut path = String::new();
    for (index, color) in code.to_colors().into_iter().enumerate() {
        if color == Color::Dark {
            use std::fmt::Write as _;
            let _ = write!(path, "M{} {}h1v1h-1z", index % width, index / width);
        }
    }
    let size = width + 4;
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-2 -2 {size} {size}" shape-rendering="crispEdges"><rect x="-2" y="-2" width="{size}" height="{size}" fill="white"/><path d="{path}" fill="black"/></svg>"#
    );
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg)
    ))
}

#[cfg(not(test))]
fn lan_ip() -> String {
    use std::net::{ToSocketAddrs, UdpSocket};
    let routed = UdpSocket::bind(("0.0.0.0", 0))
        .and_then(|socket| {
            socket.connect(("8.8.8.8", 80))?;
            socket.local_addr()
        })
        .map(|address| address.ip());
    if let Ok(ip) = routed {
        if !ip.is_loopback() {
            return ip.to_string();
        }
    }
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|host| {
            (host.as_str(), 0)
                .to_socket_addrs()
                .ok()?
                .find(|address| address.is_ipv4() && !address.ip().is_loopback())
                .map(|address| address.ip().to_string())
        })
        .unwrap_or_else(|| "127.0.0.1".into())
}

fn handle(mut stream: TcpStream, token: &str, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let Ok(input) = read_request(&mut stream) else {
        return;
    };
    let request = String::from_utf8_lossy(&input);
    let Some(line) = request.lines().next() else {
        return;
    };
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let page = format!("/{token}/");
    let api = format!("/api/{token}/");
    if method == "GET" && target == page {
        respond(
            &mut stream,
            200,
            "text/html; charset=utf-8",
            &remote_html(token),
        );
    } else if method == "GET" && target == format!("{page}manifest.webmanifest") {
        let manifest = serde_json::json!({
            "name": "Pirate Cinema Remote",
            "short_name": "PC Remote",
            "description": "Локальный пульт Pirate Cinema",
            "start_url": page,
            "scope": page,
            "display": "standalone",
            "background_color": "#050505",
            "theme_color": "#050505",
            "icons": [{"src": format!("/{token}/icon.png"), "sizes": "512x512", "type": "image/png", "purpose": "any maskable"}]
        })
        .to_string();
        respond(&mut stream, 200, "application/manifest+json", &manifest);
    } else if method == "GET" && target == format!("{page}icon.png") {
        respond_bytes(
            &mut stream,
            200,
            "image/png",
            include_bytes!("../../public/favicon.png"),
        );
    } else if method == "GET" && target == format!("{api}status") {
        let body = shared
            .status
            .lock()
            .ok()
            .and_then(|status| serde_json::to_string(&*status).ok())
            .unwrap_or_else(|| "{}".into());
        respond(&mut stream, 200, "application/json", &body);
    } else if method == "POST" && target == format!("{api}command") {
        let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
        match serde_json::from_str::<RemoteCommand>(body) {
            Ok(command) => {
                if let Ok(mut commands) = shared.commands.lock() {
                    commands.push(command);
                }
                respond(&mut stream, 204, "text/plain", "");
            }
            Err(_) => respond(&mut stream, 400, "text/plain", "bad command"),
        }
    } else {
        respond(&mut stream, 404, "text/plain", "not found");
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>, ()> {
    const LIMIT: usize = 16 * 1024;
    const DEADLINE: Duration = Duration::from_secs(2);
    let mut input = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    let started = Instant::now();
    loop {
        if started.elapsed() >= DEADLINE {
            return Err(());
        }
        let size = match stream.read(&mut chunk) {
            Ok(size) => size,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => return Err(()),
        };
        if size == 0 {
            break;
        }
        input.extend_from_slice(&chunk[..size]);
        if input.len() > LIMIT {
            return Err(());
        }
        let Some(header_end) = input.windows(4).position(|part| part == b"\r\n\r\n") else {
            continue;
        };
        let header_end = header_end + 4;
        let headers = String::from_utf8_lossy(&input[..header_end]);
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if input.len() >= header_end + content_length {
            return Ok(input);
        }
    }
    Ok(input)
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    respond_bytes(stream, status, content_type, body.as_bytes());
}

fn respond_bytes(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let label = if status == 200 {
        "OK"
    } else if status == 204 {
        "No Content"
    } else if status == 400 {
        "Bad Request"
    } else {
        "Not Found"
    };
    let response = format!(
        "HTTP/1.1 {status} {label}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

fn remote_html(token: &str) -> String {
    REMOTE_HTML.replace("__TOKEN__", token)
}

const REMOTE_HTML: &str = r##"<!doctype html><html lang="ru"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover"><meta name="theme-color" content="#050505"><link rel="manifest" href="/__TOKEN__/manifest.webmanifest"><link rel="icon" href="/__TOKEN__/icon.png"><link rel="apple-touch-icon" href="/__TOKEN__/icon.png"><meta name="apple-mobile-web-app-capable" content="yes"><meta name="apple-mobile-web-app-status-bar-style" content="black"><title>Pirate Cinema — пульт</title><style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;padding:22px 16px;background:#050505;color:#eee;font:16px system-ui}.app{max-width:520px;margin:auto}.head{display:flex;align-items:center;justify-content:space-between;margin-bottom:22px}.head strong{font-size:20px}.dot{color:#5ddd8b}.card{padding:18px;border:1px solid #292929;border-radius:16px;background:#101010}.title{min-height:48px;font-weight:700;font-size:18px}.time{display:flex;justify-content:space-between;color:#888;font-size:13px}input[type=range]{width:100%;accent-color:#287cff}.primary{display:grid;grid-template-columns:1fr 1.2fr 1fr;gap:12px;margin:22px 0}.actions{display:grid;grid-template-columns:1fr 1fr;gap:10px}button,select{min-height:50px;border:1px solid #333;border-radius:12px;background:#191919;color:#eee;font:700 15px system-ui}button:active{transform:scale(.96)}button:disabled,input:disabled,select:disabled{opacity:.45}#toggle{background:#eee;color:#080808;font-size:20px}.volume{margin:20px 0}.label{display:flex;justify-content:space-between;color:#aaa;font-size:13px}select{width:100%;padding:0 12px;margin-top:12px}.empty{color:#888;text-align:center;padding:14px 0}.offline{color:#ff6b6b}
</style></head><body><main class="app"><div class="head"><strong>Pirate Cinema</strong><span id="state" class="dot">● Пульт включён</span></div><section class="card"><div id="title" class="title">Плеер не запущен</div><input id="seek" type="range" min="0" max="1" value="0"><div class="time"><span id="now">0:00</span><span id="duration">0:00</span></div><div class="primary"><button data-command='{"action":"seek_relative","seconds":-10}'>−10</button><button id="toggle" data-command='{"action":"toggle"}'>▶</button><button data-command='{"action":"seek_relative","seconds":10}'>+10</button></div><div class="actions"><button data-command='{"action":"next"}'>Следующая серия</button><button data-command='{"action":"fullscreen"}'>На весь экран</button></div><div class="volume"><div class="label"><span>Громкость</span><span id="volumeText">100%</span></div><input id="volume" type="range" min="0" max="1" step=".01" value="1"></div><select id="audio" aria-label="Озвучка"><option value="">Озвучка недоступна</option></select><div id="empty" class="empty">Запустите видео на компьютере</div></section></main><script>
const token='__TOKEN__',api=`/api/${token}/`,fmt=n=>{n=Math.max(0,Number(n)||0);return `${Math.floor(n/60)}:${String(Math.floor(n%60)).padStart(2,'0')}`},send=body=>fetch(api+'command',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)}).catch(()=>offline());let seeking=false,changingVolume=false;document.querySelectorAll('[data-command]').forEach(button=>button.onclick=()=>send(JSON.parse(button.dataset.command)));seek.oninput=()=>{seeking=true;now.textContent=fmt(seek.value)};seek.onchange=()=>{send({action:'seek_absolute',seconds:Number(seek.value)});seeking=false};volume.oninput=()=>{changingVolume=true;volumeText.textContent=Math.round(volume.value*100)+'%'};volume.onchange=()=>{send({action:'volume',value:Number(volume.value)});changingVolume=false};audio.onchange=()=>send({action:'audio',value:audio.value});function offline(){state.textContent='● Нет связи';state.className='offline'}async function poll(){try{const response=await fetch(api+'status',{cache:'no-store'});if(!response.ok)throw 0;const s=await response.json();state.textContent='● Подключено';state.className='dot';title.textContent=s.available?s.title:'Плеер не запущен';empty.hidden=s.available;document.querySelectorAll('button,input').forEach(control=>control.disabled=!s.available);toggle.textContent=s.playing?'❚❚':'▶';if(!seeking){seek.max=Math.max(1,s.duration||1);seek.value=s.current_time||0}now.textContent=fmt(seek.value);duration.textContent=fmt(s.duration);if(!changingVolume){volume.value=s.volume??1;volumeText.textContent=Math.round(volume.value*100)+'%'}const signature=JSON.stringify(s.audio||[]);if(audio.dataset.signature!==signature){audio.dataset.signature=signature;audio.replaceChildren(...(s.audio?.length?s.audio.map(x=>new Option(x.label,x.value)):[new Option('Озвучка недоступна','')]));}audio.value=s.selected_audio||'';audio.disabled=!s.available||!s.audio?.length}catch(_){offline()}setTimeout(poll,700)}poll();
</script></body></html>"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parser_rejects_unknown_actions() {
        assert_eq!(
            serde_json::from_str::<RemoteCommand>(r#"{"action":"seek_relative","seconds":10}"#)
                .unwrap(),
            RemoteCommand::SeekRelative { seconds: 10.0 }
        );
        assert!(serde_json::from_str::<RemoteCommand>(r#"{"action":"launch"}"#).is_err());
    }

    #[test]
    fn remote_page_contains_only_the_session_token() {
        let html = remote_html("abc123");
        assert!(html.contains("const token='abc123'"));
        assert!(!html.contains("__TOKEN__"));
        assert!(!html.contains("magnet"));
        assert!(html.contains("manifest.webmanifest"));
    }

    #[test]
    fn server_requires_token_and_queues_commands() {
        stop();
        let info = start().unwrap();
        let address_and_path = info.url.strip_prefix("http://").unwrap();
        let (address, path) = address_and_path.split_once('/').unwrap();
        let path = path.trim_end_matches('/');
        let request = |target: &str, body: &str| {
            let mut stream = TcpStream::connect(address).unwrap();
            let method = if body.is_empty() { "GET" } else { "POST" };
            write!(
                stream,
                "{method} /{target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        };
        assert!(request("wrong", "").starts_with("HTTP/1.1 404"));
        assert!(request(&format!("{path}/"), "").starts_with("HTTP/1.1 200"));
        assert!(request(&format!("{path}/manifest.webmanifest"), "").starts_with("HTTP/1.1 200"));
        assert!(
            request(&format!("api/{path}/command"), r#"{"action":"toggle"}"#)
                .starts_with("HTTP/1.1 204")
        );
        assert_eq!(take_commands(), vec![RemoteCommand::Toggle]);

        let body = r#"{"action":"toggle"}"#;
        let mut split = TcpStream::connect(address).unwrap();
        write!(
            split,
            "POST /api/{path}/command HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(80));
        split.write_all(body.as_bytes()).unwrap();
        let mut response = String::new();
        split.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 204"));

        let mut stalled = TcpStream::connect(address).unwrap();
        stalled.write_all(b"P").unwrap();
        std::thread::sleep(Duration::from_millis(80));
        let started = Instant::now();
        stop();
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn token_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let root =
            std::env::temp_dir().join(format!("pc-remote-token-{}", random_token().unwrap()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("remote-token");
        write_token(&path, "0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
