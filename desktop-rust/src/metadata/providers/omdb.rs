use super::{MetadataProvider, ProviderRecord};
use crate::metadata::models::{ImageCandidate, ImageType, MediaType, MetadataSource};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub struct OmdbProvider {
    key: String,
}
impl OmdbProvider {
    pub fn from_env() -> Option<Self> {
        std::env::var("OMDB_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| Self { key })
    }
}
impl MetadataProvider for OmdbProvider {
    fn source(&self) -> MetadataSource {
        MetadataSource::Omdb
    }
    fn search(&self, query: &str, media_type: MediaType) -> Result<Option<ProviderRecord>, String> {
        let kind = if matches!(media_type, MediaType::Series) {
            "series"
        } else {
            "movie"
        };
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .user_agent("PirateCinema-Rust metadata")
            .build()
            .into();
        let mut response = agent
            .get(format!(
                "https://www.omdbapi.com/?apikey={}&t={}&type={kind}&plot=full",
                enc(&self.key),
                enc(query)
            ))
            .call()
            .map_err(|error| error.to_string())?;
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|error| error.to_string())?;
        let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
        if value.get("Response").and_then(Value::as_str) != Some("True") {
            return Ok(None);
        }
        let poster = value
            .get("Poster")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://"))
            .map(str::to_owned);
        Ok(Some(ProviderRecord {
            title: value
                .get("Title")
                .and_then(Value::as_str)
                .map(str::to_owned),
            original_title: None,
            year: value
                .get("Year")
                .and_then(Value::as_str)
                .and_then(|year| year.get(..4))
                .and_then(|year| year.parse().ok()),
            overview: value
                .get("Plot")
                .and_then(Value::as_str)
                .filter(|text| *text != "N/A" && !text.trim().is_empty())
                .map(str::to_owned),
            rating: value
                .get("imdbRating")
                .and_then(Value::as_str)
                .and_then(|rating| rating.parse().ok()),
            images: poster
                .into_iter()
                .map(|url| ImageCandidate {
                    source: MetadataSource::Omdb,
                    image_type: ImageType::Poster,
                    url,
                    language: None,
                    width: None,
                    height: None,
                    vote_average: None,
                    vote_count: None,
                })
                .collect(),
            imdb_id: value
                .get("imdbID")
                .and_then(Value::as_str)
                .map(str::to_owned),
            tmdb_id: None,
            tvmaze_id: None,
            genres: value
                .get("Genre")
                .and_then(Value::as_str)
                .map(|genres| genres.split(", ").map(str::to_owned).collect())
                .unwrap_or_default(),
        }))
    }
}
fn enc(value: &str) -> String {
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
