mod embedded;
mod library;
#[path = "../../desktop-rust/src/rutube_prototype.rs"]
mod online;
mod platform;
mod state;

#[cfg(all(not(feature = "mobile"), feature = "desktop"))]
use dioxus::desktop as native;
#[cfg(feature = "mobile")]
use dioxus::mobile as native;

use dioxus::prelude::*;
use pirate_cinema_core::{
    self as core,
    history::{HistoryStore, MediaMetadata},
    SearchResult, Torrent, VideoFile,
};
use std::path::Path;
use std::sync::Arc;

async fn embedded_engine(
    profile: Result<std::path::PathBuf, String>,
    mut state: Signal<Option<Arc<embedded::Engine>>>,
) -> Result<Arc<embedded::Engine>, String> {
    if let Some(engine) = state.peek().clone() {
        return Ok(engine);
    }
    let engine = embedded::Engine::start(&profile?).await?;
    state.set(Some(engine.clone()));
    Ok(engine)
}

const STYLE: &str = include_str!("style.css");

fn main() {
    dioxus::launch(App)
}

async fn background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

fn store(profile: &Path) -> Result<HistoryStore, String> {
    library::store(profile)
}

#[component]
fn App() -> Element {
    let profile = use_hook(platform::data_dir);
    let poster_profile = profile.clone();
    native::use_asset_handler("posters", move |request, responder| {
        let bytes = poster_profile
            .as_ref()
            .ok()
            .and_then(|p| library::poster_bytes(p, request.uri().path()));
        let status = if bytes.is_some() {
            native::wry::http::StatusCode::OK
        } else {
            native::wry::http::StatusCode::NOT_FOUND
        };
        let mut response = native::wry::http::Response::new(bytes.unwrap_or_default());
        *response.status_mut() = status;
        response.headers_mut().insert(
            "Content-Type",
            native::wry::http::HeaderValue::from_static("image/jpeg"),
        );
        response.headers_mut().insert(
            "Cache-Control",
            native::wry::http::HeaderValue::from_static("no-store"),
        );
        responder.respond(response);
    });
    let mut page = use_signal(|| "online");
    let online_active = use_memo(move || page() == "online");
    let local_profile = profile.clone();
    let mut library = use_signal(move || {
        local_profile
            .ok()
            .and_then(|p| library::recent_cards(&p).ok())
            .unwrap_or_default()
    });
    let mut results = use_signal(Vec::<SearchResult>::new);
    let engine = use_signal(|| None::<Arc<embedded::Engine>>);
    let mut query = use_signal(String::new);
    let mut magnet = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut enriching = use_signal(|| false);
    let mut metadata_query = use_signal(String::new);
    let mut message = use_signal(String::new);
    let mut selected = use_signal(|| None::<Torrent>);
    let mut files = use_signal(Vec::<VideoFile>::new);
    let mut details = use_signal(|| None::<MediaMetadata>);
    let mut metadata_revision = use_signal(|| 0u64);
    let metadata_profile = profile.clone();
    let card_metadata = use_memo(move || {
        let _revision = metadata_revision();
        let mut cards = std::collections::BTreeMap::new();
        if let Some(db) = metadata_profile.as_ref().ok().and_then(|p| store(p).ok()) {
            for item in library() {
                if let Ok(Some(value)) = db.metadata(&item.hash) {
                    cards.insert(item.hash, value);
                }
            }
        }
        cards
    });
    let pending_profile = profile.clone();
    let mut pending = use_signal(move || {
        pending_profile
            .ok()
            .and_then(|p| state::load_pending(&p).ok().flatten())
    });
    let mut has_mpv = use_signal(|| false);
    let mut history_revision = use_signal(|| 0u64);

    use_effect(move || {
        has_mpv.set(platform::installed(core::mobile::MPV_PACKAGE).unwrap_or(false));
    });

    let progress_profile = profile.clone();
    use_future(move || {
        let profile = progress_profile.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let Some(result) = platform::take_player_result() else {
                    continue;
                };
                let Some((hash, file, url)) = pending.peek().clone() else {
                    continue;
                };
                if !state::matches_result(&url, result.source_url.as_deref()) {
                    continue;
                }
                match (&profile, result.progress_seconds()) {
                    (Ok(path), Ok(Some((position, duration)))) => {
                        if let Err(error) = store(path).and_then(|db| {
                            if db.get(&hash, file.id).map_err(|e| e.to_string())?.is_none() {
                                db.mark_played(&hash, file.id, &file.name, Some(&file.path))
                                    .map_err(|e| e.to_string())?;
                            }
                            db.save_progress(&hash, file.id, position, duration)
                                .map_err(|e| e.to_string())
                                .and_then(|saved| {
                                    if saved {
                                        Ok(())
                                    } else {
                                        Err("Позиция не была сохранена".into())
                                    }
                                })
                        }) {
                            message.set(error);
                            platform::retry_player_result(result);
                        } else {
                            let _ = state::clear_pending(path);
                            pending.set(None);
                            history_revision += 1;
                            message.set(format!(
                                "Сохранено: {}:{:02}",
                                position / 60,
                                position % 60
                            ))
                        }
                    }
                    (_, Err(error)) => {
                        if let Ok(path) = &profile {
                            let _ = state::clear_pending(path);
                        }
                        pending.set(None);
                        message.set(error)
                    }
                    _ => {
                        if let Ok(path) = &profile {
                            let _ = state::clear_pending(path);
                        }
                        pending.set(None);
                        message.set("Плеер не вернул позицию; прежняя история сохранена".into())
                    }
                }
            }
        }
    });

    rsx! {
        style{ "{STYLE}" }
        main{
            header{
                strong{"Pirate Cinema"}
                span{"librqbit"}
            }
            if let Err(error)=&profile{p{role:"alert","{error}"}}
            if !message().is_empty(){p{class:"notice",role:"status","{message}"}}
            if pending().is_some(){
                p{class:"notice","Ожидается возврат позиции из MPV."}
                button{class:"secondary",onclick:{let profile=profile.clone();move |_|{
                    if let Ok(path)=&profile{
                        match state::clear_pending(path){
                            Ok(())=>{pending.set(None);message.set("Ожидание отменено. Прежняя история сохранена; MPV не закрыт.".into())},
                            Err(error)=>message.set(error)
                        }
                    }
                }},"Отменить ожидание плеера"}
            }
            section{class:"content",
                // Keep the existing player component mounted across navigation.
                div{style:if page()=="online"{"display:block"}else{"display:none"},
                    online::RutubePage{active:online_active}
                }
                if page()=="online"{}else{
                if let Some(item)=selected(){
                    button{class:"secondary",onclick:move |_|{selected.set(None);files.set(Vec::new());details.set(None);},"← Назад"}
                    h1{"{item.title}"}
                    label{r#for:"metadata-query","Название для поиска метаданных"}
                    input{id:"metadata-query",value:"{metadata_query}",disabled:enriching(),oninput:move|event|metadata_query.set(event.value())}
                    button{class:"secondary",disabled:busy()||enriching()||metadata_query().trim().chars().count()<2,onclick:{let item=item.clone();let profile=profile.clone();move |_|{
                        let title=metadata_query();let hash=item.hash.clone();let selected_hash=hash.clone();let series=files().len()>1;let profile=profile.clone();enriching.set(true);message.set("Поиск описания и постера…".into());
                        spawn(async move{
                            match background(move||library::refresh(&profile?,&hash,&title,series)).await{
                                Ok((movie,warning))=>{metadata_revision+=1;if selected.peek().as_ref().is_some_and(|current|current.hash==selected_hash){details.set(Some(movie));message.set(warning.unwrap_or_else(||"Описание и постер сохранены на устройстве".into()))}},
                                Err(error)=>{if selected.peek().as_ref().is_some_and(|current|current.hash==selected_hash){message.set(error)}}
                            }enriching.set(false);
                        });
                    }},"Найти описание и постер"}
                    small{"По этой кнопке название передаётся Cinemeta, TVmaze, Wikidata и Wikipedia. История не передаётся."}
                    if enriching(){small{role:"status","Поиск метаданных… Воспроизведение доступно."}}
                    if let Some(movie)=details(){
                        div{class:"description",
                            if movie.poster_file.is_some(){Poster{key:"{item.hash}-{metadata_revision}",hash:item.hash.clone(),revision:metadata_revision(),title:movie.title.clone()}}
                            div{h2{"{movie.title}"}if let Some(overview)=movie.overview{p{"{overview}"}}}
                        }
                    }
                    if busy()&&files().is_empty(){p{"Загрузка файлов…"}}
                    for file in files(){
                        {
                            let resume=profile.as_ref().ok().and_then(|p|store(p).ok()).and_then(|db|db.get(&item.hash,file.id).ok().flatten()).and_then(|entry|entry.playback_timecode).unwrap_or(0);
                            let hash=item.hash.clone();let title=item.title.clone();let file_name=file.name.clone();let profile=profile.clone();
                            rsx!{article{class:"file",key:"{file.id}-{history_revision}",
                                div{strong{"{file_name}"}if resume>0{small{"Продолжить с {resume / 60}:{resume % 60:02}"}}}
                                button{disabled:busy()||pending().is_some(),onclick:move |_|{
                                    if pending.peek().is_some(){return}
                                    let hash=hash.clone();let file=file.clone();let title=title.clone();let profile=profile.clone();busy.set(true);
                                    spawn(async move{let result=async{
                                        if !platform::installed(core::mobile::MPV_PACKAGE)?{return Err("Установите mpv-android в настройках".into())}
                                        let url=embedded_engine(profile.clone(),engine).await?.play_url(&hash,&file).await?;
                                        let mut request=platform::playback_request(url,&title,resume.saturating_mul(1000))?;
                                        let session=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
                                        let separator=if request.url.contains('?'){'&'}else{'?'};
                                        request.url.push_str(&format!("{separator}pc_session={session}"));
                                        let path=profile.as_ref().map_err(Clone::clone)?;
                                        let db=store(path)?;
                                        state::save_pending(path,&hash,&file,&request.url)?;
                                        pending.set(Some((hash.clone(),file.clone(),request.url.clone())));
                                        if let Err(error)=platform::play(&request){let _=state::clear_pending(path);pending.set(None);return Err(error)}
                                        db.mark_played(&hash,file.id,&file.name,Some(&file.path)).map_err(|e|e.to_string())?;
                                        Ok(())
                                    }.await;
                                    if let Err(error)=result{message.set(error)}busy.set(false);});
                                },"Запустить"}
                            }}
                        }
                    }
                }else if page()=="settings"{
                    h1{"Настройки"}
                    article{h2{"Торрент-движок"}p{"Встроенный librqbit. Отдельный сервер не нужен. Загрузка выбранного видео начинается по кнопке «Запустить»."}}
                    article{h2{"Приложения для просмотра"}
                        button{onclick:move |_|{if let Err(error)=platform::open_dependency(core::mobile::MPV_PACKAGE,"https://github.com/mpv-android/mpv-android/releases/latest"){message.set(error)}},if has_mpv(){"Открыть MPV"}else{"Установить MPV"}}
                        button{class:"secondary",onclick:move |_|has_mpv.set(platform::installed(core::mobile::MPV_PACKAGE).unwrap_or(false)),"Проверить MPV"}
                        p{"История хранится локально в SQLite. Точная позиция сохраняется, только если внешний плеер вернул её."}
                    }
                }else{
                    h1{if page()=="search"{"Поиск"}else{"Медиатека"}}
                    if page()=="search"{
                        p{"Встроенный librqbit · поиск по локальному публичному индексу. Первый поиск загружает около 51 МБ. Название и история не отправляются в индекс."}
                        label{r#for:"query","Фильм или сериал"}
                        input{id:"query",value:"{query}",oninput:move|event|query.set(event.value())}
                        button{disabled:busy()||query().trim().len()<2,onclick:{let profile=profile.clone();move |_|{
                            let text=query();let profile=profile.clone();busy.set(true);message.set("librqbit: запускаем движок и читаем поисковый индекс… Первый поиск может занять до 90 секунд.".into());
                            spawn(async move{
                                let result=match embedded_engine(profile.clone(),engine).await {
                                    Ok(_)=>background(move||embedded::search(&profile?,&text)).await,
                                    Err(error)=>Err(error)
                                };
                                match result{Ok(items)=>{message.set(format!("Найдено: {}",items.len()));results.set(items)},Err(e)=>message.set(e)}busy.set(false);
                            });
                        }},"Найти"}
                        for item in results(){
                            article{key:"{item.hash}",strong{"{item.title}"}small{"{item.size} · Сидов: {item.seeders}"}
                                button{disabled:busy(),onclick:{let profile=profile.clone();move |_|{
                                    let magnet=item.magnet.clone();let profile=profile.clone();busy.set(true);
                                    spawn(async move{let result=async{let engine=embedded_engine(profile,engine).await?;engine.add(&magnet).await?;library.set(engine.list());Ok::<_,String>(())}.await;match result{Ok(_)=>message.set("Добавлено. Загрузка видео начнётся только по кнопке «Запустить».".into()),Err(e)=>message.set(e)}busy.set(false);});
                                }},"Добавить"}
                            }
                        }
                    }else{
                        button{disabled:busy(),onclick:{let profile=profile.clone();move |_|{
                            busy.set(true);let profile=profile.clone();
                            spawn(async move{match embedded_engine(profile,engine).await{Ok(e)=>{library.set(e.list());message.set(String::new())},Err(e)=>message.set(e)}busy.set(false);});
                        }},if busy(){"Обновление…"}else{"Обновить медиатеку"}}
                        details{summary{"Добавить magnet-ссылку"}
                            label{r#for:"magnet","Magnet-ссылка"}
                            textarea{id:"magnet",value:"{magnet}",oninput:move|event|magnet.set(event.value())}
                            button{disabled:busy(),onclick:{let profile=profile.clone();move |_|{
                                let value=magnet();busy.set(true);let profile=profile.clone();
                                spawn(async move{let result=async{let engine=embedded_engine(profile,engine).await?;engine.add(&value).await?;library.set(engine.list());Ok::<_,String>(())}.await;match result{Ok(_)=>{magnet.set(String::new());message.set("Добавлено".into())},Err(e)=>message.set(e)}busy.set(false);});
                            }},"Добавить"}
                        }
                        p{"Встроенный librqbit. Видео загружается в хранилище приложения только после явного запуска; для MKV используется отдельный MPV."}
                        if library().is_empty(){p{"Пока нет сохранённых карточек."}}
                        div{class:"library-grid",for item in library(){
                            {let cached=card_metadata().get(&item.hash).cloned();let title=cached.as_ref().map_or_else(||item.title.clone(),|m|m.title.clone());let profile=profile.clone();
                            rsx!{article{class:"library-card",key:"{item.hash}",button{class:"card",disabled:busy(),onclick:move |_|{
                                let item=item.clone();let selected_item=item.clone();busy.set(true);message.set(String::new());metadata_query.set(cached.as_ref().map_or_else(||item.title.clone(),|m|m.title.clone()));details.set(cached.clone());selected.set(Some(item.clone()));let profile=profile.clone();
                                spawn(async move{
                                    let result=embedded_engine(profile,engine).await.and_then(|engine|engine.files(&item.hash));
                                    match result{
                                        Ok(value)=>{if selected.peek().as_ref().is_none_or(|current|current.hash!=selected_item.hash){busy.set(false);return}files.set(value);busy.set(false);},Err(e)=>{message.set(e);busy.set(false)}
                                    }
                                });
                            },Poster{key:"{item.hash}-{metadata_revision}",hash:item.hash.clone(),revision:metadata_revision(),title:title.clone()}strong{"{title}"}}}}
                            }
                        }}
                    }
                }}
            }
            nav{
                for (id,title) in [("online","Онлайн"),("library","Медиатека"),("search","Раздачи"),("settings","Настройки")]{
                    button{class:if page()==id{"active"}else{""},disabled:busy(),onclick:move |_|{page.set(id);selected.set(None);details.set(None);files.set(Vec::new());},"{title}"}
                }
            }
        }
    }
}

#[component]
fn Poster(hash: String, revision: u64, title: String) -> Element {
    let mut failed = use_signal(|| false);
    rsx! {div{class:"poster",
        if failed(){span{"Нет постера"}}else{
            img{src:format!("/posters/{hash}.jpg?v={revision}"),alt:title,loading:"lazy",onerror:move |_|failed.set(true)}
        }
    }}
}
