use super::{MetadataProvider, ProviderRecord};
use crate::metadata::models::{ImageCandidate, ImageType, MediaType, MetadataSource};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub struct TmdbProvider {
    key: String,
}

impl TmdbProvider {
    pub fn from_env() -> Option<Self> {
        std::env::var("TMDB_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| Self { key })
    }
    fn get(&self, url: &str) -> Result<Value, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(6)))
            .user_agent("PirateCinema-Rust metadata")
            .build()
            .into();
        let mut response = agent.get(url).call().map_err(|error| error.to_string())?;
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&body).map_err(|error| error.to_string())
    }
    fn encode(value: &str) -> String {
        value
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (byte as char).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect()
    }
}

impl MetadataProvider for TmdbProvider {
    fn source(&self) -> MetadataSource {
        MetadataSource::Tmdb
    }
    fn search(&self, query: &str, media_type: MediaType) -> Result<Option<ProviderRecord>, String> {
        let kind = if matches!(media_type, MediaType::Series) {
            "tv"
        } else {
            "movie"
        };
        let search = self.get(&format!(
            "https://api.themoviedb.org/3/search/{kind}?api_key={}&language=ru-RU&query={}",
            Self::encode(&self.key),
            Self::encode(query)
        ))?;
        let Some(result) = search.pointer("/results/0") else {
            return Ok(None);
        };
        let id = result
            .get("id")
            .and_then(Value::as_i64)
            .ok_or("TMDB вернул результат без ID")?;
        let detail = self.get(&format!("https://api.themoviedb.org/3/{kind}/{id}?api_key={}&language=ru-RU&append_to_response=external_ids,images", Self::encode(&self.key)))?;
        let mut images = Vec::new();
        for poster in detail
            .pointer("/images/posters")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(path) = poster.get("file_path").and_then(Value::as_str) {
                images.push(ImageCandidate {
                    source: MetadataSource::Tmdb,
                    image_type: ImageType::Poster,
                    url: format!("https://image.tmdb.org/t/p/w780{path}"),
                    language: poster
                        .get("iso_639_1")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    width: poster
                        .get("width")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                    height: poster
                        .get("height")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                    vote_average: poster.get("vote_average").and_then(Value::as_f64),
                    vote_count: poster
                        .get("vote_count")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                });
            }
        }
        for backdrop in detail
            .pointer("/images/backdrops")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(path) = backdrop.get("file_path").and_then(Value::as_str) {
                images.push(ImageCandidate {
                    source: MetadataSource::Tmdb,
                    image_type: ImageType::Backdrop,
                    url: format!("https://image.tmdb.org/t/p/w1280{path}"),
                    language: backdrop
                        .get("iso_639_1")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    width: backdrop
                        .get("width")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                    height: backdrop
                        .get("height")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                    vote_average: backdrop.get("vote_average").and_then(Value::as_f64),
                    vote_count: backdrop
                        .get("vote_count")
                        .and_then(Value::as_u64)
                        .and_then(|value| value.try_into().ok()),
                });
            }
        }
        Ok(Some(ProviderRecord {
            title: detail
                .get("title")
                .or_else(|| detail.get("name"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            original_title: detail
                .get("original_title")
                .or_else(|| detail.get("original_name"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            year: detail
                .get("release_date")
                .or_else(|| detail.get("first_air_date"))
                .and_then(Value::as_str)
                .and_then(|value| value.get(..4))
                .and_then(|value| value.parse().ok()),
            overview: detail
                .get("overview")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned),
            rating: detail.get("vote_average").and_then(Value::as_f64),
            images,
            imdb_id: detail
                .pointer("/external_ids/imdb_id")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            tmdb_id: Some(id),
            tvmaze_id: None,
            genres: detail
                .get("genres")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|genre| genre.get("name").and_then(Value::as_str).map(str::to_owned))
                .collect(),
        }))
    }
}
