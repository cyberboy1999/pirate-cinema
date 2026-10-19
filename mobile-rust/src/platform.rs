use pirate_cinema_core::mobile::{PlaybackRequest, PlaybackResult};
use std::path::PathBuf;
use std::sync::Mutex;

static PLAYER_RESULT: Mutex<Option<PlaybackResult>> = Mutex::new(None);

pub fn playback_request(
    url: String,
    title: &str,
    position_ms: i64,
) -> Result<PlaybackRequest, String> {
    let uri: ureq::http::Uri = url.parse().map_err(|_| "Некорректный локальный поток")?;
    if uri.scheme_str() != Some("http")
        || uri.host() != Some("127.0.0.1")
        || uri.port_u16().is_none_or(|port| port == 0)
        || !uri.path().starts_with("/torrents/")
    {
        return Err("Разрешён только локальный поток встроенного движка".into());
    }
    if title.trim().is_empty() || title.contains('\0') {
        return Err("Укажите название видео".into());
    }
    let position_ms = i32::try_from(position_ms)
        .ok()
        .filter(|p| *p >= 0)
        .ok_or("Некорректная позиция воспроизведения")?;
    Ok(PlaybackRequest {
        package: pirate_cinema_core::mobile::MPV_PACKAGE,
        url,
        title: title.trim().into(),
        position_ms,
    })
}

pub fn take_player_result() -> Option<PlaybackResult> {
    PLAYER_RESULT.lock().ok()?.take()
}

pub fn retry_player_result(result: PlaybackResult) {
    if let Ok(mut slot) = PLAYER_RESULT.lock() {
        if slot.is_none() {
            *slot = Some(result);
        }
    }
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::objects::{JObject, JValue};

    fn with_activity<T>(
        callback: impl FnOnce(&mut jni::JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> Result<T, String> {
        let context = ndk_context::android_context();
        // ndk-context owns these references for the running Wry activity.
        let vm =
            unsafe { jni::JavaVM::from_raw(context.vm().cast()) }.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
        let activity = unsafe { JObject::from_raw(context.context().cast()) };
        let result = callback(&mut env, &activity);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            return Err("Android не смог открыть приложение. Проверьте установку MPV.".into());
        }
        result.map_err(|e| e.to_string())
    }

    pub fn data_dir() -> Result<PathBuf, String> {
        with_activity(|env, activity| {
            let directory = env
                .call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])?
                .l()?;
            let path = env
                .call_method(directory, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                .l()?;
            let path: String = env.get_string(&jni::objects::JString::from(path))?.into();
            Ok(PathBuf::from(path))
        })
    }

    pub fn installed(package: &str) -> Result<bool, String> {
        with_activity(|env, activity| {
            let manager = env
                .call_method(
                    activity,
                    "getPackageManager",
                    "()Landroid/content/pm/PackageManager;",
                    &[],
                )?
                .l()?;
            let package = JObject::from(env.new_string(package)?);
            let intent = env
                .call_method(
                    manager,
                    "getLaunchIntentForPackage",
                    "(Ljava/lang/String;)Landroid/content/Intent;",
                    &[JValue::Object(&package)],
                )?
                .l()?;
            Ok(!intent.is_null())
        })
    }

    pub fn open_dependency(package: &str, url: &str) -> Result<(), String> {
        with_activity(|env, activity| {
            let manager = env
                .call_method(
                    activity,
                    "getPackageManager",
                    "()Landroid/content/pm/PackageManager;",
                    &[],
                )?
                .l()?;
            let package = JObject::from(env.new_string(package)?);
            let mut intent = env
                .call_method(
                    manager,
                    "getLaunchIntentForPackage",
                    "(Ljava/lang/String;)Landroid/content/Intent;",
                    &[JValue::Object(&package)],
                )?
                .l()?;
            if intent.is_null() {
                let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
                let url = JObject::from(env.new_string(url)?);
                let uri = env
                    .call_static_method(
                        "android/net/Uri",
                        "parse",
                        "(Ljava/lang/String;)Landroid/net/Uri;",
                        &[JValue::Object(&url)],
                    )?
                    .l()?;
                intent = env.new_object(
                    "android/content/Intent",
                    "(Ljava/lang/String;Landroid/net/Uri;)V",
                    &[JValue::Object(&action), JValue::Object(&uri)],
                )?;
            }
            env.call_method(
                activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })
    }

    pub fn play(request: &PlaybackRequest) -> Result<(), String> {
        with_activity(|env, activity| {
            let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&action)],
            )?;
            let url = JObject::from(env.new_string(&request.url)?);
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&url)],
                )?
                .l()?;
            let mime = JObject::from(env.new_string("video/*")?);
            env.call_method(
                &intent,
                "setDataAndType",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&uri), JValue::Object(&mime)],
            )?;
            let package = JObject::from(env.new_string(request.package)?);
            env.call_method(
                &intent,
                "setPackage",
                "(Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&package)],
            )?;
            let key = JObject::from(env.new_string("position")?);
            env.call_method(
                &intent,
                "putExtra",
                "(Ljava/lang/String;I)Landroid/content/Intent;",
                &[JValue::Object(&key), JValue::Int(request.position_ms)],
            )?;
            let key = JObject::from(env.new_string("title")?);
            let title = JObject::from(env.new_string(&request.title)?);
            env.call_method(
                &intent,
                "putExtra",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&key), JValue::Object(&title)],
            )?;
            env.call_method(
                activity,
                "startActivityForResult",
                "(Landroid/content/Intent;I)V",
                &[JValue::Object(&intent), JValue::Int(7001)],
            )?;
            Ok(())
        })
    }

    #[no_mangle]
    pub extern "system" fn Java_dev_dioxus_main_MainActivity_playerResult(
        mut env: jni::JNIEnv,
        _activity: JObject,
        position: jni::sys::jlong,
        duration: jni::sys::jlong,
        completed: jni::sys::jboolean,
        source: jni::objects::JString,
    ) {
        let source_url = if source.is_null() {
            None
        } else {
            env.get_string(&source).ok().map(String::from)
        };
        if let Ok(mut result) = PLAYER_RESULT.lock() {
            *result = Some(PlaybackResult {
                source_url,
                position_ms: (position >= 0).then_some(position),
                duration_ms: (duration > 0).then_some(duration),
                completed: completed != 0,
            });
        }
    }
}

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(not(target_os = "android"))]
pub fn data_dir() -> Result<PathBuf, String> {
    std::env::var_os("PIRATE_CINEMA_DATA_DIR").map(PathBuf::from).ok_or("Укажите изолированный PIRATE_CINEMA_DATA_DIR для настольного теста мобильного интерфейса".into())
}
#[cfg(not(target_os = "android"))]
pub fn installed(_package: &str) -> Result<bool, String> {
    Ok(false)
}
#[cfg(not(target_os = "android"))]
pub fn open_dependency(_package: &str, _url: &str) -> Result<(), String> {
    Err("Доступно на Android".into())
}
#[cfg(not(target_os = "android"))]
pub fn play(_request: &PlaybackRequest) -> Result<(), String> {
    Err("Запуск MPV через Android Intent доступен на Android".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_engine_loopback_streams_and_preserves_resume() {
        let url = "http://127.0.0.1:12345/torrents/abc/stream/0";
        let request = playback_request(url.into(), " Фильм ", 43000).unwrap();
        assert_eq!(request.url, url);
        assert_eq!(request.title, "Фильм");
        assert_eq!(request.position_ms, 43000);
        for invalid in [
            "file:///private/video.mp4",
            "https://example.com/torrents/x",
            "http://127.0.0.1:12345/other",
            "http://127.0.0.1/torrents/x",
        ] {
            assert!(playback_request(invalid.into(), "Фильм", 0).is_err());
        }
        for position in [-1, i64::MAX] {
            assert!(playback_request(url.into(), "Фильм", position).is_err());
        }
    }
    #[test]
    fn android_manifest_queries_only_the_player_and_ui_uses_no_configured_endpoint() {
        let manifest = include_str!("../Dioxus.toml");
        assert_eq!(manifest.matches("<package ").count(), 1);
        assert!(manifest.contains("is.xyz.mpv"));
        let ui = include_str!("main.rs");
        for retired in [
            "server_enabled",
            "load_endpoint",
            "save_endpoint",
            "8090",
            "8091",
        ] {
            assert!(
                !ui.contains(retired),
                "Unexpected legacy connection path: {retired}"
            );
        }
    }
}
