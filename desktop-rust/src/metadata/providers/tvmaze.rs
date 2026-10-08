use super::{MetadataProvider, ProviderRecord};
use crate::metadata::models::{ImageCandidate, ImageType, MediaType, MetadataSource};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub struct TvMazeProvider;

fn strip_html(value: &str) -> String {
    let mut output = String::new();
    let mut tag = false;
    for character in value.chars() {
        match character {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}

impl MetadataProvider for TvMazeProvider {
    fn source(&self) -> MetadataSource {
        MetadataSource::TvMaze
    }
    fn search(&self, query: &str, media_type: MediaType) -> Result<Option<ProviderRecord>, String> {
        if !matches!(media_type, MediaType::Series) {
            return Ok(None);
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .user_agent("PirateCinema-Rust metadata")
            .build()
            .into();
        let mut response = agent
            .get(format!(
                "https://api.tvmaze.com/search/shows?q={}",
                url_encode(query)
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
        let Some(show) = value.as_array().and_then(|rows| {
            rows.iter().filter_map(|row| row.get("show")).find(|show| {
                let name = show.get("name").and_then(Value::as_str).unwrap_or("");
                crate::catalog::title_matches(query, name)
            })
        }) else {
            return Ok(None);
        };
        let image = show
            .pointer("/image/original")
            .or_else(|| show.pointer("/image/medium"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(Some(ProviderRecord {
            title: show.get("name").and_then(Value::as_str).map(str::to_owned),
            original_title: show.get("name").and_then(Value::as_str).map(str::to_owned),
            year: show
                .get("premiered")
                .and_then(Value::as_str)
                .and_then(|date| date.get(..4))
                .and_then(|year| year.parse().ok()),
            overview: show
                .get("summary")
                .and_then(Value::as_str)
                .map(strip_html)
                .filter(|value| !value.is_empty()),
            rating: show.pointer("/rating/average").and_then(Value::as_f64),
            images: image
                .into_iter()
                .map(|url| ImageCandidate {
                    source: MetadataSource::TvMaze,
                    image_type: ImageType::Poster,
                    url,
                    language: None,
                    width: None,
                    height: None,
                    vote_average: None,
                    vote_count: None,
                })
                .collect(),
            imdb_id: None,
            tvmaze_id: show.get("id").and_then(Value::as_i64),
            genres: show
                .get("genres")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        }))
    }
}

fn url_encode(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::strip_html;
    #[test]
    fn strips_html() {
        assert_eq!(strip_html("<p>One &amp; <b>two</b></p>"), "One & two");
    }
}
