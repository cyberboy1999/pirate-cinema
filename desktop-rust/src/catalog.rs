use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

const CINEMETA: &str = "https://v3-cinemeta.strem.io";
const WIKIDATA: &str = "https://query.wikidata.org/sparql";
type WikidataRecord = (String, Option<String>, Option<String>);
type WikidataRecords = std::collections::HashMap<String, WikidataRecord>;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Movie {
    pub id: String,
    pub title: String,
    pub original_title: String,
    pub year: Option<i64>,
    pub rating: Option<f64>,
    pub poster_url: Option<String>,
    #[serde(default)]
    pub alternate_poster_url: Option<String>,
    #[serde(default)]
    pub background_url: Option<String>,
    pub overview: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
}

fn agent() -> ureq::Agent {
    // Metadata refresh is an interactive action. A stalled public provider must
    // not hold the whole library sync hostage for a dozen seconds per request.
    agent_with_timeout(Duration::from_secs(5))
}

fn agent_with_timeout(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

fn json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    let mut response = agent
        .get(url)
        .header("accept", "application/json")
        .header("user-agent", "PirateCinema-Rust/0.1.6 (local desktop app)")
        .call()
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    let read_error = response
        .body_mut()
        .as_reader()
        .read_to_end(&mut bytes)
        .err()
        .map(|error| error.to_string());
    parse_json_body(&bytes, read_error)
}

fn parse_json_body(bytes: &[u8], read_error: Option<String>) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| read_error.unwrap_or_else(|| error.to_string()))
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

fn year(value: &Value) -> Option<i64> {
    let text = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    text.as_bytes()
        .windows(4)
        .find(|digits| {
            matches!(&digits[..2], b"19" | b"20") && digits.iter().all(u8::is_ascii_digit)
        })
        .and_then(|digits| std::str::from_utf8(digits).ok()?.parse().ok())
}

fn valid_imdb(value: &str) -> bool {
    value.strip_prefix("tt").is_some_and(|digits| {
        !digits.is_empty() && digits.len() <= 12 && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn parse_catalog(value: &Value) -> Vec<Movie> {
    value
        .get("metas")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(30)
        .filter_map(parse_catalog_item)
        .collect()
}

fn parse_catalog_item(item: &Value) -> Option<Movie> {
    let id = item.get("imdb_id").or_else(|| item.get("id"))?.as_str()?;
    let title = item.get("name").or_else(|| item.get("title"))?.as_str()?;
    if !valid_imdb(id) || title.trim().is_empty() {
        return None;
    }
    Some(Movie {
        id: id.to_owned(),
        title: title.to_owned(),
        original_title: title.to_owned(),
        year: item
            .get("releaseInfo")
            .or_else(|| item.get("year"))
            .and_then(year),
        rating: item
            .get("imdbRating")
            .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok())),
        poster_url: item
            .get("poster")
            .and_then(Value::as_str)
            .map(str::to_owned),
        alternate_poster_url: None,
        background_url: item
            .get("background")
            .and_then(Value::as_str)
            .map(str::to_owned),
        overview: item
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        genres: item
            .get("genres")
            .or_else(|| item.get("genre"))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .take(12)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn wikidata_records(agent: &ureq::Agent, ids: &[&str]) -> Result<WikidataRecords, String> {
    let ids = ids
        .iter()
        .copied()
        .filter(|id| valid_imdb(id))
        .take(30)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let values = ids
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let query = format!("SELECT ?id ?label ?description ?image WHERE {{ VALUES ?id {{ {values} }} ?item <http://www.wikidata.org/prop/direct/P345> ?id. OPTIONAL {{ ?item <http://www.w3.org/2000/01/rdf-schema#label> ?label. FILTER(LANG(?label)=\"ru\") }} OPTIONAL {{ ?item <http://schema.org/description> ?description. FILTER(LANG(?description)=\"ru\") }} OPTIONAL {{ ?item <http://www.wikidata.org/prop/direct/P18> ?image. }} }}");
    let payload = json(
        agent,
        &format!("{WIKIDATA}?format=json&query={}", encode(&query)),
    )?;
    Ok(parse_wikidata_records(&payload))
}

fn parse_wikidata_records(payload: &Value) -> WikidataRecords {
    let mut matches = std::collections::HashMap::new();
    if let Some(rows) = payload
        .pointer("/results/bindings")
        .and_then(Value::as_array)
    {
        for row in rows {
            let Some(id) = row.pointer("/id/value").and_then(Value::as_str) else {
                continue;
            };
            if !valid_imdb(id) {
                continue;
            }
            let label = row
                .pointer("/label/value")
                .and_then(Value::as_str)
                .unwrap_or("");
            let description = row
                .pointer("/description/value")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let image = row
                .pointer("/image/value")
                .and_then(Value::as_str)
                .and_then(wikidata_image_url);
            matches.insert(id.to_owned(), (label.trim().to_owned(), description, image));
        }
    }
    matches
}

fn wikidata_title_match(
    agent: &ureq::Agent,
    title: &str,
) -> Result<Option<(String, WikidataRecord)>, String> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
    let query = format!("SELECT ?id ?label ?description ?image WHERE {{ ?item <http://www.w3.org/2000/01/rdf-schema#label> \"{escaped}\"@ru; <http://www.wikidata.org/prop/direct/P345> ?id. OPTIONAL {{ ?item <http://schema.org/description> ?description. FILTER(LANG(?description)=\"ru\") }} OPTIONAL {{ ?item <http://www.wikidata.org/prop/direct/P18> ?image. }} BIND(\"{escaped}\" AS ?label) }} LIMIT 1");
    Ok(parse_wikidata_records(&json(
        agent,
        &format!("{WIKIDATA}?format=json&query={}", encode(&query)),
    )?)
    .into_iter()
    .next())
}

fn cinemeta_by_id(agent: &ureq::Agent, id: &str, kind: &str) -> Option<Movie> {
    json(agent, &format!("{CINEMETA}/meta/{kind}/{id}.json"))
        .ok()?
        .get("meta")
        .and_then(parse_catalog_item)
}

fn wikidata_image_url(value: &str) -> Option<String> {
    let file = value.rsplit('/').next()?.replace('_', " ");
    (!file.is_empty()).then(|| {
        format!(
            "https://www.wikidata.org/wiki/Special:FilePath/{}",
            encode(&file)
        )
    })
}

pub fn popular() -> Result<Vec<Movie>, String> {
    popular_kind("movie")
}

pub fn popular_series() -> Result<Vec<Movie>, String> {
    popular_kind("series")
}

fn popular_kind(kind: &str) -> Result<Vec<Movie>, String> {
    if !matches!(kind, "movie" | "series") {
        return Err("Неизвестный тип каталога".into());
    }
    let agent = agent_with_timeout(Duration::from_secs(5));
    let mut items = parse_catalog(&json(
        &agent,
        &format!("{CINEMETA}/catalog/{kind}/top.json"),
    )?);
    if items.is_empty() {
        return Err("Cinemeta вернула пустой каталог".into());
    }
    let ids = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    if let Ok(records) = wikidata_records(&agent, &ids) {
        for item in &mut items {
            if let Some((name, description, image)) = records.get(&item.id) {
                if !name.is_empty() {
                    item.title = name.clone();
                }
                if item.overview.is_none() {
                    item.overview = description.clone();
                }
                item.alternate_poster_url = image.clone();
            }
        }
    }
    Ok(items)
}

pub fn fallback_popular() -> Vec<Movie> {
    [
        ("tt0111161", "Побег из Шоушенка", 1994, 9.3),
        ("tt0068646", "Крёстный отец", 1972, 9.2),
        ("tt0468569", "Тёмный рыцарь", 2008, 9.0),
        (
            "tt0167260",
            "Властелин колец: Возвращение короля",
            2003,
            9.0,
        ),
        ("tt0108052", "Список Шиндлера", 1993, 9.0),
        ("tt0050083", "12 разгневанных мужчин", 1957, 9.0),
        ("tt0110912", "Криминальное чтиво", 1994, 8.9),
        ("tt0120737", "Властелин колец: Братство Кольца", 2001, 8.9),
        ("tt0060196", "Хороший, плохой, злой", 1966, 8.8),
        ("tt0109830", "Форрест Гамп", 1994, 8.8),
        ("tt0137523", "Бойцовский клуб", 1999, 8.8),
        ("tt1375666", "Начало", 2010, 8.8),
    ]
    .into_iter()
    .map(|(id, title, year, rating)| Movie {
        id: id.to_owned(),
        title: title.to_owned(),
        original_title: title.to_owned(),
        year: Some(year),
        rating: Some(rating),
        poster_url: None,
        alternate_poster_url: None,
        background_url: None,
        overview: None,
        genres: Vec::new(),
    })
    .collect()
}

pub fn cached_popular(cache: &Path) -> Vec<Movie> {
    cached_catalog(cache, "popular")
}

pub fn cached_popular_series(cache: &Path) -> Vec<Movie> {
    cached_catalog(cache, "popular-series")
}

fn cached_catalog(cache: &Path, name: &str) -> Vec<Movie> {
    let Ok(bytes) = std::fs::read(cache.join(format!("{name}.json"))) else {
        return Vec::new();
    };
    let Ok(items) = serde_json::from_slice::<Vec<Movie>>(&bytes) else {
        return Vec::new();
    };
    items
        .into_iter()
        .take(30)
        .filter(|item| valid_imdb(&item.id) && !item.title.trim().is_empty())
        .collect()
}

pub fn cache_popular(cache: &Path, items: &[Movie]) -> Result<(), String> {
    cache_catalog(cache, "popular", items)
}

pub fn cache_popular_series(cache: &Path, items: &[Movie]) -> Result<(), String> {
    cache_catalog(cache, "popular-series", items)
}

fn cache_catalog(cache: &Path, name: &str, items: &[Movie]) -> Result<(), String> {
    std::fs::create_dir_all(cache).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(items).map_err(|error| error.to_string())?;
    std::fs::write(cache.join(format!("{name}.json")), bytes).map_err(|error| error.to_string())
}

pub fn cached_poster(cache: &Path, id: &str) -> Option<Vec<u8>> {
    valid_imdb(id)
        .then(|| {
            let path = cache.join(format!("{id}.jpg"));
            (std::fs::metadata(&path).ok()?.len() <= 5 * 1024 * 1024)
                .then(|| std::fs::read(path).ok())
                .flatten()
                .filter(|bytes| image::load_from_memory(bytes).is_ok())
        })
        .flatten()
}

pub fn cache_poster(cache: &Path, id: &str, bytes: &[u8]) -> Result<(), String> {
    if !valid_imdb(id) || bytes.len() > 5 * 1024 * 1024 || image::load_from_memory(bytes).is_err() {
        return Err("Некорректный постер каталога".into());
    }
    std::fs::create_dir_all(cache).map_err(|error| error.to_string())?;
    std::fs::write(cache.join(format!("{id}.jpg")), bytes).map_err(|error| error.to_string())
}

pub fn clean_title(raw: &str) -> (String, Option<i64>) {
    let first = raw.split(" / ").next().unwrap_or(raw);
    let mut words = Vec::new();
    let mut release_year = year(&Value::String(raw.to_owned()));
    for word in first.split(['.', '_', '[', ']', '(', ')', '{', '}', ' ']) {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        if release_marker(word) {
            break;
        }
        if word.len() == 4 && word.bytes().all(|byte| byte.is_ascii_digit()) {
            let parsed = word.parse::<i64>().ok();
            if parsed.is_some_and(|year| (1900..=2100).contains(&year)) {
                release_year = parsed;
                break;
            }
        }
        if [
            "2160p", "1080p", "720p", "480p", "webrip", "web-dl", "bdrip", "bluray", "remux",
            "x264", "x265", "h264", "h265",
        ]
        .iter()
        .any(|tag| word.eq_ignore_ascii_case(tag))
        {
            break;
        }
        words.push(word);
    }
    let clean = words.join(" ").trim().to_owned();
    (
        if clean.is_empty() {
            raw.trim().to_owned()
        } else {
            clean
        },
        release_year,
    )
}

fn release_marker(word: &str) -> bool {
    let word = word.trim_matches(['-', '+', ',']).to_ascii_lowercase();
    ["сезон", "серия", "season", "episode"]
        .iter()
        .any(|marker| word == *marker)
        || word
            .strip_prefix('s')
            .or_else(|| word.strip_prefix('e'))
            .is_some_and(|suffix| {
                suffix
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            })
        || word.strip_prefix('х').is_some_and(|suffix| {
            suffix
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
}

fn normalized(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn same_release(left: &str, right: &str) -> bool {
    let (left, left_year) = clean_title(left);
    let (right, right_year) = clean_title(right);
    normalized(&left) == normalized(&right)
        && match (left_year, right_year) {
            (Some(left), Some(right)) => (left - right).abs() <= 1,
            _ => true,
        }
}

fn tvmaze(agent: &ureq::Agent, title: &str, wanted_year: Option<i64>) -> Option<Movie> {
    let url = format!("https://api.tvmaze.com/search/shows?q={}", encode(title));
    let rows = json(agent, &url).ok()?.as_array()?.clone();
    rows.into_iter().find_map(|row| {
        let show = row.get("show")?;
        let name = show.get("name")?.as_str()?;
        let year = show
            .get("premiered")
            .and_then(Value::as_str)
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse::<i64>().ok());
        if wanted_year.is_some_and(|wanted| year.is_some_and(|found| (found - wanted).abs() > 1)) {
            return None;
        }
        Some(Movie {
            id: format!("tvmaze:{}", show.get("id")?.as_i64()?),
            title: name.to_owned(),
            original_title: name.to_owned(),
            year: year.or(wanted_year),
            rating: show.pointer("/rating/average").and_then(Value::as_f64),
            poster_url: show
                .pointer("/image/original")
                .or_else(|| show.pointer("/image/medium"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            alternate_poster_url: None,
            background_url: None,
            overview: show
                .get("summary")
                .and_then(Value::as_str)
                .map(strip_html)
                .filter(|text| !text.is_empty()),
            genres: show
                .get("genres")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        })
    })
}

fn strip_html(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut tag = false;
    for character in value.chars() {
        match character {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}

pub fn lookup(title: &str, series: bool) -> Result<Option<Movie>, String> {
    let (clean, wanted_year) = clean_title(title);
    if clean.chars().count() < 2 || clean.chars().count() > 120 {
        return Ok(None);
    }
    let agent = agent();
    let titles = title_candidates(title);
    let kinds = if series {
        ["series", "movie"]
    } else {
        ["movie", "series"]
    };
    let mut match_item = None;
    let mut provider_responded = false;
    'search: for query in &titles {
        for kind in kinds {
            let url = format!(
                "{CINEMETA}/catalog/{kind}/top/search={}.json",
                encode(query)
            );
            let Ok(result) = json(&agent, &url) else {
                // All remaining attempts target the same public service. Retrying
                // it with another title/type only multiplies the timeout.
                break 'search;
            };
            provider_responded = true;
            let candidates = parse_catalog(&result);
            match_item = candidates.into_iter().find(|candidate| {
                normalized(&candidate.title) == normalized(query)
                    && wanted_year.is_none_or(|year| {
                        candidate.year.is_none_or(|found| (found - year).abs() <= 1)
                    })
            });
            if match_item.is_some() {
                break 'search;
            }
        }
    }
    let Some(mut item) = match_item else {
        if let Ok(Some((id, (name, description, image)))) = wikidata_title_match(&agent, &clean) {
            if let Some(mut item) = kinds
                .iter()
                .find_map(|kind| cinemeta_by_id(&agent, &id, kind))
            {
                if !name.is_empty() {
                    item.title = name;
                }
                item.overview = description.or(item.overview);
                item.alternate_poster_url = image;
                return Ok(Some(item));
            }
            return Ok(Some(Movie {
                id,
                title: if name.is_empty() { clean.clone() } else { name },
                original_title: clean,
                year: wanted_year,
                rating: None,
                poster_url: None,
                alternate_poster_url: image,
                background_url: None,
                overview: description,
                genres: Vec::new(),
            }));
        }
        if series {
            if let Some(item) = tvmaze(&agent, &clean, wanted_year) {
                return Ok(Some(item));
            }
        }
        return if provider_responded {
            Ok(None)
        } else {
            Err("Cinemeta недоступна".into())
        };
    };
    if let Ok(records) = wikidata_records(&agent, &[&item.id]) {
        if let Some((name, description, image)) = records.get(&item.id) {
            if !name.is_empty() {
                item.title = name.clone();
            }
            if item.overview.is_none() {
                item.overview = description.clone();
            }
            if item.alternate_poster_url.is_none() {
                item.alternate_poster_url = image.clone();
            }
        }
    }
    Ok(Some(item))
}

pub fn title_candidates(title: &str) -> Vec<String> {
    let mut titles = title
        .split(" / ")
        .take(2)
        .map(|part| clean_title(part).0)
        .filter(|name| (2..=120).contains(&name.chars().count()))
        .collect::<Vec<_>>();
    titles.reverse(); // The original title is usually the second half and matches Cinemeta.
    titles.dedup_by(|left, right| normalized(left) == normalized(right));
    titles
}

pub fn movie_poster_jpeg(movie: &Movie, _series: bool) -> Result<Vec<u8>, String> {
    let mut urls = Vec::new();
    if let Some(primary) = movie.poster_url.as_deref() {
        urls.push(primary.to_owned());
    }
    if let Some(alternate) = movie.alternate_poster_url.as_deref() {
        if !urls.iter().any(|url| url == alternate) {
            urls.push(alternate.to_owned());
        }
    }
    for url in urls {
        if let Ok(bytes) = poster_jpeg(&url) {
            return Ok(bytes);
        }
    }
    Err("Не удалось получить обложку из Cinemeta, TVmaze или Wikidata".into())
}

pub fn poster_jpeg(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err("Постер должен загружаться по HTTPS".into());
    }
    let mut response = agent().get(url).call().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(5 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err("Постер слишком большой".into());
    }
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    if image.width() > 4096 || image.height() > 4096 {
        return Err("Постер слишком большой".into());
    }
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Jpeg)
        .map_err(|error| error.to_string())?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parser_keeps_only_valid_movies() {
        let value = serde_json::json!({"metas":[{"id":"tt123","name":"Film","releaseInfo":"2025","imdbRating":"7.3","poster":"https://example.com/a.jpg","background":"https://example.com/background.jpg","genres":["Драма","Триллер"]},{"id":"bad","name":"Wrong"}]});
        let items = parse_catalog(&value);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].year, Some(2025));
        assert_eq!(items[0].rating, Some(7.3));
        assert_eq!(items[0].genres, ["Драма", "Триллер"]);
        assert_eq!(
            items[0].background_url.as_deref(),
            Some("https://example.com/background.jpg")
        );
    }

    #[test]
    fn webp_posters_can_be_decoded() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 3)
            .write_to(&mut bytes, image::ImageFormat::WebP)
            .unwrap();
        let decoded = image::load_from_memory(bytes.get_ref()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 3));
    }

    #[test]
    fn release_title_removes_year_and_technical_suffix() {
        assert_eq!(
            clean_title("Дюна.2024.1080p.WEB-DL"),
            ("Дюна".into(), Some(2024))
        );
        assert_eq!(
            clean_title("Дюна / Dune (2021) 1080p"),
            ("Дюна".into(), Some(2021))
        );
        assert_eq!(
            title_candidates("Тачки / Cars (2006) BDRip"),
            ["Cars", "Тачки"]
        );
        assert!(same_release("Дюна.2021.1080p", "Дюна (2021) BDRip"));
        assert!(!same_release("Дюна (1984)", "Дюна (2021)"));
        assert!(!same_release("Дюна (2021)", "Прибытие (2021)"));
    }

    #[test]
    fn release_title_stops_before_series_bundle_markers() {
        assert_eq!(
            clean_title("Мажор [S01-05 + Мажор. Фильм] (2014-2025) WEB-DL"),
            ("Мажор".into(), Some(2014))
        );
        assert_eq!(
            title_candidates("Рик и Морти / Rick and Morty [S01-09] (2013-2026) BDRip"),
            ["Rick and Morty", "Рик и Морти"]
        );
    }

    #[test]
    fn wikidata_records_keep_localized_fields() {
        let payload = serde_json::json!({"results":{"bindings":[{
            "id":{"value":"tt123"},
            "label":{"value":"Фильм"},
            "description":{"value":"Описание"},
            "image":{"value":"https://commons.wikimedia.org/wiki/Special:FilePath/Test_film.jpg"}
        }]}});
        let records = parse_wikidata_records(&payload);
        assert_eq!(
            records.get("tt123"),
            Some(&(
                "Фильм".into(),
                Some("Описание".into()),
                Some("https://www.wikidata.org/wiki/Special:FilePath/Test%20film.jpg".into())
            ))
        );
    }

    #[test]
    fn tvmaze_summary_is_plain_text() {
        assert_eq!(strip_html("<p>One &amp; <b>two</b></p>"), "One & two");
    }

    #[test]
    fn complete_json_survives_a_late_chunked_body_timeout() {
        assert_eq!(
            parse_json_body(br#"{"metas":[]}"#, Some("timed out".into())).unwrap(),
            serde_json::json!({"metas": []})
        );
        assert_eq!(
            parse_json_body(b"{", Some("timed out".into())).unwrap_err(),
            "timed out"
        );
    }

    #[test]
    #[ignore = "uses public Cinemeta, TVmaze and Wikidata"]
    fn live_series_bundle_gets_metadata_and_poster() {
        let movie = lookup(
            "Мажор [S01-05 + Мажор. Фильм + Мажор в Сочи] (2014-2025) WEB-DL",
            true,
        )
        .unwrap()
        .expect("Мажор must resolve through a public source");
        assert!(movie.overview.is_some());
    }

    #[test]
    #[ignore = "uses the public Cinemeta service"]
    fn live_cinemeta_metadata_is_readable() {
        let payload = json(
            &agent(),
            "https://v3-cinemeta.strem.io/meta/movie/tt0111161.json",
        )
        .unwrap();
        assert_eq!(
            payload.pointer("/meta/id").and_then(Value::as_str),
            Some("tt0111161")
        );
    }

    #[test]
    fn cached_catalog_rejects_corrupt_content_and_unsafe_ids() {
        let cache = std::env::temp_dir().join(format!(
            "pirate-catalog-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(cached_popular(&cache).is_empty());
        assert!(cache_poster(&cache, "../bad", b"bad").is_err());
        let item = Movie {
            id: "tt123".into(),
            title: "Фильм".into(),
            original_title: "Film".into(),
            year: Some(2025),
            rating: None,
            poster_url: None,
            alternate_poster_url: None,
            background_url: None,
            overview: None,
            genres: vec![],
        };
        cache_popular(&cache, &[item]).unwrap();
        assert_eq!(cached_popular(&cache)[0].title, "Фильм");
        let series = Movie {
            id: "tt456".into(),
            title: "Сериал".into(),
            original_title: "Series".into(),
            year: Some(2025),
            rating: None,
            poster_url: None,
            alternate_poster_url: None,
            background_url: None,
            overview: None,
            genres: vec![],
        };
        cache_popular_series(&cache, &[series]).unwrap();
        assert_eq!(cached_popular_series(&cache)[0].title, "Сериал");
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        cache_poster(&cache, "tt123", jpeg.get_ref()).unwrap();
        assert_eq!(cached_poster(&cache, "tt123"), Some(jpeg.into_inner()));
        std::fs::write(cache.join("popular.json"), b"{").unwrap();
        assert!(cached_popular(&cache).is_empty());
        std::fs::remove_file(cache.join("popular.json")).unwrap();
        std::fs::remove_file(cache.join("popular-series.json")).unwrap();
        std::fs::remove_file(cache.join("tt123.jpg")).unwrap();
        std::fs::remove_dir(cache).unwrap();
    }

    #[test]
    fn offline_home_catalog_has_valid_russian_cards() {
        let items = fallback_popular();
        assert!(items.len() >= 12);
        assert!(items.iter().all(|item| valid_imdb(&item.id)));
        assert!(items.iter().all(|item| item.poster_url.is_none()));
        assert!(items
            .iter()
            .any(|item| item.title.chars().any(|ch| ('А'..='я').contains(&ch))));
    }
}
