use super::{MetadataProvider, ProviderRecord};
use crate::metadata::models::{ImageCandidate, ImageType, MediaType, MetadataSource};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub struct FanartProvider {
    key: String,
}
impl FanartProvider {
    pub fn from_env() -> Option<Self> {
        std::env::var("FANART_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| Self { key })
    }
    pub fn images_for_tmdb(
        &self,
        id: i64,
        media_type: MediaType,
    ) -> Result<Vec<ImageCandidate>, String> {
        let kind = if matches!(media_type, MediaType::Series) {
            "tv"
        } else {
            "movies"
        };
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(5)))
            .user_agent("PirateCinema-Rust metadata")
            .build()
            .into();
        let mut response = agent
            .get(format!(
                "https://webservice.fanart.tv/v3/{kind}/{id}?api_key={}",
                enc(&self.key)
            ))
            .call()
            .map_err(|error| error.to_string())?;
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|error| error.to_string())?;
        let payload: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
        let mut images = Vec::new();
        for (field, image_type) in [
            ("movieposter", ImageType::Poster),
            ("tvposter", ImageType::Poster),
            ("moviebackground", ImageType::Backdrop),
            ("showbackground", ImageType::Backdrop),
            ("movielogo", ImageType::Logo),
            ("hdtvlogo", ImageType::Logo),
            ("moviebanner", ImageType::Banner),
            ("tvbanner", ImageType::Banner),
        ] {
            for image in payload
                .get(field)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(url) = image
                    .get("url")
                    .and_then(Value::as_str)
                    .filter(|url| url.starts_with("https://"))
                {
                    images.push(ImageCandidate {
                        source: MetadataSource::Fanart,
                        image_type,
                        url: url.to_owned(),
                        language: image
                            .get("lang")
                            .and_then(Value::as_str)
                            .filter(|lang| !lang.is_empty())
                            .map(str::to_owned),
                        width: None,
                        height: None,
                        vote_average: image
                            .get("likes")
                            .and_then(Value::as_str)
                            .and_then(|likes| likes.parse().ok()),
                        vote_count: image
                            .get("likes")
                            .and_then(Value::as_str)
                            .and_then(|likes| likes.parse().ok()),
                    });
                }
            }
        }
        Ok(images)
    }
}
impl MetadataProvider for FanartProvider {
    fn source(&self) -> MetadataSource {
        MetadataSource::Fanart
    }
    fn search(&self, _: &str, _: MediaType) -> Result<Option<ProviderRecord>, String> {
        Ok(None)
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
