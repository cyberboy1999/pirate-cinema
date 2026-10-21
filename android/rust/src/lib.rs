//! Embedded librqbit engine for the Android client.

use std::{
    fs::File,
    io::{BufReader, Read},
    net::{Ipv4Addr, SocketAddr},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{Mutex, OnceLock, mpsc},
    thread,
};

use anyhow::{Context, Result};
use flate2::read::DeflateDecoder;
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{JNI_FALSE, JNI_TRUE, jboolean, jstring},
};
use librqbit::http_api::{HttpApi, HttpApiOptions};
use librqbit::{Api, Session, SessionOptions, SessionPersistenceConfig};
use librqbit_dualstack_sockets::{BindOpts, TcpListener};
use serde::{
    Deserialize, Serialize,
    de::{Deserializer, Error, SeqAccess, Visitor},
};
use tokio::sync::oneshot;

const HTTP_PORT: u16 = 3030;
const MAX_SEARCH_INDEX_BYTES: u64 = 80 * 1024 * 1024;
const MAX_SEARCH_JSON_BYTES: u64 = 320 * 1024 * 1024;
const MAX_JSON_STRING_BYTES: usize = 16 * 1024;
const MAX_NAMES: usize = 64;

struct LimitedJsonRead<R> {
    inner: R,
    total: u64,
    in_string: bool,
    escaped: bool,
    string_bytes: usize,
}
impl<R: Read> Read for LimitedJsonRead<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.total = self.total.saturating_add(count as u64);
        if self.total > MAX_SEARCH_JSON_BYTES {
            return Err(std::io::Error::other("search JSON is too large"));
        }
        for byte in &buffer[..count] {
            if self.in_string {
                if self.escaped {
                    self.escaped = false;
                } else if *byte == b'\\' {
                    self.escaped = true;
                } else if *byte == b'"' {
                    self.in_string = false;
                    self.string_bytes = 0;
                    continue;
                }
                self.string_bytes += 1;
                if self.string_bytes > MAX_JSON_STRING_BYTES {
                    return Err(std::io::Error::other("search JSON string is too large"));
                }
            } else if *byte == b'"' {
                self.in_string = true;
                self.string_bytes = 0;
            }
        }
        Ok(count)
    }
}

#[derive(Default, Serialize)]
struct BoundedNames(Vec<String>);
impl<'de> Deserialize<'de> for BoundedNames {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NamesVisitor;
        impl<'de> Visitor<'de> for NamesVisitor {
            type Value = BoundedNames;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("null or a bounded names array")
            }
            fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
                Ok(BoundedNames::default())
            }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(BoundedNames::default())
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut names = Vec::new();
                while let Some(name) = seq.next_element()? {
                    if names.len() == MAX_NAMES {
                        return Err(A::Error::custom("too many alternate names"));
                    }
                    names.push(name);
                }
                Ok(BoundedNames(names))
            }
        }
        deserializer.deserialize_any(NamesVisitor)
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "PascalCase")]
struct SearchItem {
    title: String,
    name: String,
    names: BoundedNames,
    size: String,
    create_date: String,
    tracker: String,
    magnet: String,
    hash: String,
    #[serde(rename = "IMDBID")]
    imdbid: String,
    year: i32,
    peer: i32,
    seed: i32,
    video_quality: i32,
    #[serde(skip)]
    score: i32,
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn search_index(path: &std::path::Path, query: &str) -> Result<String> {
    struct Matches<'a> {
        query: &'a str,
        tokens: Vec<&'a str>,
    }
    impl<'de> Visitor<'de> for Matches<'_> {
        type Value = Vec<SearchItem>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a RuTor array")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut found = Vec::new();
            while let Some(mut item) = seq.next_element::<SearchItem>()? {
                let searchable = normalize(&format!(
                    "{} {} {} {}",
                    item.title,
                    item.name,
                    item.names.0.join(" "),
                    item.year
                ));
                if self.tokens.iter().all(|token| searchable.contains(token))
                    && item.hash.len() == 40
                    && item.hash.chars().all(|c| c.is_ascii_hexdigit())
                    && item.magnet.to_ascii_lowercase().starts_with("magnet:?")
                {
                    item.score = if searchable.contains(self.query) {
                        10_000
                    } else {
                        0
                    } + item.seed.saturating_mul(10).saturating_add(item.peer);
                    found.push(item);
                    if found.len() > 40 {
                        rank_results(&mut found);
                        found.truncate(40);
                    }
                }
            }
            Ok(found)
        }
    }
    let normalized = normalize(query);
    if normalized.chars().count() < 2 {
        anyhow::bail!("search query must contain at least two letters or digits");
    }
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_SEARCH_INDEX_BYTES {
        anyhow::bail!("compressed search index is too large");
    }
    let reader = BufReader::with_capacity(
        256 * 1024,
        LimitedJsonRead {
            inner: DeflateDecoder::new(file),
            total: 0,
            in_string: false,
            escaped: false,
            string_bytes: 0,
        },
    );
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let mut matches = deserializer.deserialize_seq(Matches {
        query: &normalized,
        tokens: normalized.split_whitespace().collect(),
    })?;
    rank_results(&mut matches);
    matches.truncate(40);
    Ok(serde_json::to_string(&matches)?)
}

fn rank_results(items: &mut [SearchItem]) {
    items.sort_unstable_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| b.seed.cmp(&a.seed))
            .then_with(|| b.create_date.cmp(&a.create_date))
    });
}

struct RunningEngine {
    shutdown: oneshot::Sender<()>,
    thread: thread::JoinHandle<()>,
}

static ENGINE: OnceLock<Mutex<Option<RunningEngine>>> = OnceLock::new();

fn engine_slot() -> &'static Mutex<Option<RunningEngine>> {
    ENGINE.get_or_init(|| Mutex::new(None))
}

fn start_engine(data_dir: PathBuf) -> Result<()> {
    let mut slot = engine_slot()
        .lock()
        .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
    if slot.is_some() {
        return Ok(());
    }

    // librqbit owns the CORS layer. Set its narrow WebView origin allowlist
    // before this library creates any Rust runtime worker threads.
    unsafe {
        std::env::set_var(
            "CORS_ALLOW_REGEXP",
            r"^(https?|capacitor)://localhost(?::\d+)?$",
        );
    }

    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let thread = thread::Builder::new()
        .name("librqbit-runtime".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = started_tx.send(Err(anyhow::anyhow!(error)));
                    return;
                }
            };
            runtime.block_on(async move {
                if let Err(error) = run_engine(data_dir, shutdown_rx, started_tx).await {
                    tracing::error!(?error, "librqbit engine stopped with an error");
                }
            });
        })
        .context("failed to create librqbit runtime thread")?;

    started_rx
        .recv()
        .context("librqbit runtime exited during startup")??;
    *slot = Some(RunningEngine {
        shutdown: shutdown_tx,
        thread,
    });
    Ok(())
}

async fn run_engine(
    data_dir: PathBuf,
    shutdown_rx: oneshot::Receiver<()>,
    started_tx: mpsc::SyncSender<Result<()>>,
) -> Result<()> {
    let downloads = data_dir.join("downloads");
    let state = data_dir.join("session");
    std::fs::create_dir_all(&downloads)?;
    std::fs::create_dir_all(&state)?;

    let session = Session::new_with_opts(
        downloads,
        SessionOptions {
            dht: Some(librqbit::DhtSessionConfig {
                persistence: Some(librqbit::dht::DhtPersistenceConfig {
                    config_filename: Some(data_dir.join("dht.json")),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            fastresume: true,
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(state),
            }),
            concurrent_init_limit: Some(2),
            runtime_worker_threads: Some(2),
            peer_limit: Some(80),
            ipv4_only: true,
            ..Default::default()
        },
    )
    .await
    .context("failed to initialize librqbit session")?;

    let listener = TcpListener::bind_tcp(
        SocketAddr::from((Ipv4Addr::LOCALHOST, HTTP_PORT)),
        BindOpts::default(),
    )
    .context("failed to bind librqbit HTTP server to loopback")?;
    let server = HttpApi::new(
        Api::new(session.clone(), None, None),
        Some(HttpApiOptions {
            max_upload_body_size: Some(10 * 1024 * 1024),
            ..Default::default()
        }),
    )
    .make_http_api_and_run(listener, None);

    let _ = started_tx.send(Ok(()));
    tokio::select! {
        result = server => result?,
        _ = shutdown_rx => {}
    }
    session.stop().await;
    Ok(())
}

fn stop_engine() -> Result<()> {
    let running = engine_slot()
        .lock()
        .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?
        .take();
    if let Some(running) = running {
        let _ = running.shutdown.send(());
        running
            .thread
            .join()
            .map_err(|_| anyhow::anyhow!("librqbit runtime panicked"))?;
    }
    Ok(())
}

fn jni_guard(env: &mut JNIEnv<'_>, operation: impl FnOnce() -> Result<()>) -> jboolean {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => JNI_TRUE,
        Ok(Err(error)) => {
            let _ = env.throw_new("java/lang/IllegalStateException", format!("{error:#}"));
            JNI_FALSE
        }
        Err(_) => {
            let _ = env.throw_new(
                "java/lang/IllegalStateException",
                "panic in librqbit JNI boundary",
            );
            JNI_FALSE
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_piratecinema_app_LibrqbitPlugin_nativeStart(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    data_dir: JString<'_>,
) -> jboolean {
    let path = match env.get_string(&data_dir) {
        Ok(path) => PathBuf::from(path.to_string_lossy().into_owned()),
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException", error.to_string());
            return JNI_FALSE;
        }
    };
    jni_guard(&mut env, || start_engine(path))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_piratecinema_app_LibrqbitPlugin_nativeStop(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jboolean {
    jni_guard(&mut env, stop_engine)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_piratecinema_app_TorrentSearchPlugin_nativeSearchIndex(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    path: JString<'_>,
    query: JString<'_>,
) -> jstring {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<String> {
        let path = PathBuf::from(env.get_string(&path)?.to_string_lossy().into_owned());
        let query = env.get_string(&query)?.to_string_lossy().into_owned();
        search_index(&path, &query)
    }));
    match result {
        Ok(Ok(value)) => match env.new_string(value) {
            Ok(value) => value.into_raw(),
            Err(error) => {
                let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
                std::ptr::null_mut()
            }
        },
        Ok(Err(error)) => {
            let _ = env.throw_new("java/lang/IllegalStateException", format!("{error:#}"));
            std::ptr::null_mut()
        }
        Err(_) => {
            let _ = env.throw_new(
                "java/lang/IllegalStateException",
                "panic in torrent search JNI boundary",
            );
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_idempotent_and_shutdown_releases_the_port() {
        let root = tempfile::tempdir().unwrap();
        start_engine(root.path().to_path_buf()).unwrap();
        start_engine(root.path().to_path_buf()).unwrap();
        stop_engine().unwrap();
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, HTTP_PORT)).unwrap();
        drop(listener);
    }
}
