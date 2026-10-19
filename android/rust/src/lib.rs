//! Embedded librqbit engine for the Android client.

use std::{
    net::{Ipv4Addr, SocketAddr},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{Mutex, OnceLock, mpsc},
    thread,
};

use anyhow::{Context, Result};
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{JNI_FALSE, JNI_TRUE, jboolean},
};
use librqbit::http_api::{HttpApi, HttpApiOptions};
use librqbit::{Api, Session, SessionOptions, SessionPersistenceConfig};
use librqbit_dualstack_sockets::{BindOpts, TcpListener};
use tokio::sync::oneshot;

const HTTP_PORT: u16 = 3030;

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
