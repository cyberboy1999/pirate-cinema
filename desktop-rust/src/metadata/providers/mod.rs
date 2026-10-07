use super::models::{ImageCandidate, MediaType, MetadataSource};

pub mod tvmaze;

#[derive(Clone, Debug, Default)]
pub struct ProviderRecord {
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub year: Option<i64>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub images: Vec<ImageCandidate>,
    pub imdb_id: Option<String>,
    pub tvmaze_id: Option<i64>,
    pub genres: Vec<String>,
}

pub trait MetadataProvider: Send + Sync {
    fn source(&self) -> MetadataSource;
    fn search(&self, query: &str, media_type: MediaType) -> Result<Option<ProviderRecord>, String>;
}
