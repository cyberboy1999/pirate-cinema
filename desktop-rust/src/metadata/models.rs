#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaType {
    Movie,
    Series,
}

impl MediaType {
    pub fn from_series(series: bool) -> Self {
        if series {
            Self::Series
        } else {
            Self::Movie
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExternalIds {
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<i64>,
    pub tvmaze_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataSource {
    Tmdb,
    Fanart,
    Omdb,
    TvMaze,
    Cinemeta,
    Wikipedia,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalizedText {
    pub text: String,
    pub language: Option<String>,
    pub source: MetadataSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageType {
    Poster,
    Backdrop,
    Logo,
    Banner,
    SeasonPoster,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageCandidate {
    pub source: MetadataSource,
    pub image_type: ImageType,
    pub url: String,
    pub language: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<u32>,
}

pub fn best_image(
    images: impl IntoIterator<Item = ImageCandidate>,
    kind: ImageType,
) -> Option<ImageCandidate> {
    images
        .into_iter()
        .filter(|image| image.image_type == kind && image.url.starts_with("https://"))
        .max_by_key(|image| {
            let language = match image.language.as_deref() {
                Some("ru") => 4_i64,
                Some("en") => 3,
                None | Some("") => 2,
                _ => 1,
            };
            let pixels = i64::from(image.width.unwrap_or(0)) * i64::from(image.height.unwrap_or(0));
            language * 1_000_000_000_000
                + pixels * 10
                + i64::from(image.vote_count.unwrap_or(0)).min(10_000)
        })
}
