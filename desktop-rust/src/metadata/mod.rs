pub mod models;
pub mod providers;

use crate::catalog;
pub use crate::catalog::{
    cache_popular, cache_popular_series, cache_poster, cached_popular, cached_popular_series,
    cached_poster, Movie,
};
pub use models::{
    best_image, ExternalIds, ImageCandidate, ImageType, LocalizedText, MediaType, MetadataSource,
};
use providers::{tvmaze::TvMazeProvider, MetadataProvider, ProviderRecord};

/// UI-facing metadata facade. Provider-specific HTTP calls stay behind this module.
pub fn lookup(title: &str, series: bool) -> Result<Option<Movie>, String> {
    MetadataManager.lookup(title, MediaType::from_series(series))
}
pub fn movie_poster_jpeg(movie: &Movie, series: bool) -> Result<Vec<u8>, String> {
    catalog::movie_poster_jpeg(movie, series)
}
pub fn popular() -> Result<Vec<Movie>, String> {
    catalog::popular()
}
pub fn popular_series() -> Result<Vec<Movie>, String> {
    catalog::popular_series()
}
pub fn fallback_popular() -> Vec<Movie> {
    catalog::fallback_popular()
}
pub fn same_release(left: &str, right: &str) -> bool {
    catalog::same_release(left, right)
}
pub fn title_candidates(title: &str) -> Vec<String> {
    catalog::title_candidates(title)
}

#[derive(Default)]
pub struct MetadataManager;
impl MetadataManager {
    pub fn lookup(&self, title: &str, media_type: MediaType) -> Result<Option<Movie>, String> {
        let primary = catalog::lookup(title, matches!(media_type, MediaType::Series));
        if matches!(&primary, Ok(Some(_))) || !matches!(media_type, MediaType::Series) {
            return primary;
        }
        let query = catalog::title_candidates(title)
            .into_iter()
            .next()
            .unwrap_or_else(|| title.to_owned());
        finish_lookup(primary, TvMazeProvider.search(&query, media_type))
    }
}

fn finish_lookup(
    primary: Result<Option<Movie>, String>,
    fallback: Result<Option<ProviderRecord>, String>,
) -> Result<Option<Movie>, String> {
    match fallback {
        Ok(Some(record)) => Ok(Some(movie_from_record(record))),
        Ok(None) => primary,
        Err(error) => match primary {
            Ok(Some(movie)) => Ok(Some(movie)),
            Ok(None) => Err(format!("TVmaze: {error}")),
            Err(first) => Err(format!("{first}; TVmaze: {error}")),
        },
    }
}

fn movie_from_record(record: ProviderRecord) -> Movie {
    let poster_url = best_image(record.images.clone(), ImageType::Poster).map(|image| image.url);
    let background_url = best_image(record.images, ImageType::Backdrop).map(|image| image.url);
    let id = record
        .imdb_id
        .or_else(|| record.tvmaze_id.map(|id| format!("tvmaze:{id}")))
        .unwrap_or_else(|| "metadata:unknown".into());
    Movie {
        id,
        title: record.title.unwrap_or_else(|| {
            record
                .original_title
                .clone()
                .unwrap_or_else(|| "Без названия".into())
        }),
        original_title: record.original_title.unwrap_or_default(),
        year: record.year,
        rating: record.rating,
        poster_url,
        alternate_poster_url: None,
        background_url,
        overview: record.overview,
        genres: record.genres,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_outage_is_not_reported_as_no_match() {
        assert!(finish_lookup(Err("Cinemeta недоступна".into()), Err("offline".into())).is_err());
        assert!(finish_lookup(Ok(None), Err("offline".into())).is_err());
        assert!(finish_lookup(Err("offline".into()), Ok(None)).is_err());
        let record = ProviderRecord {
            title: Some("Fallback".into()),
            ..Default::default()
        };
        assert_eq!(
            finish_lookup(Err("offline".into()), Ok(Some(record)))
                .unwrap()
                .unwrap()
                .title,
            "Fallback"
        );
    }
    #[test]
    fn ranks_russian_and_larger_posters_first() {
        let image = best_image(
            [
                ImageCandidate {
                    source: MetadataSource::TvMaze,
                    image_type: ImageType::Poster,
                    url: "https://example.test/small.jpg".into(),
                    language: None,
                    width: Some(600),
                    height: Some(900),
                    vote_average: None,
                    vote_count: None,
                },
                ImageCandidate {
                    source: MetadataSource::Cinemeta,
                    image_type: ImageType::Poster,
                    url: "https://example.test/russian.jpg".into(),
                    language: Some("ru".into()),
                    width: Some(500),
                    height: Some(750),
                    vote_average: None,
                    vote_count: None,
                },
            ],
            ImageType::Poster,
        )
        .unwrap();
        assert_eq!(image.url, "https://example.test/russian.jpg");
    }
}
