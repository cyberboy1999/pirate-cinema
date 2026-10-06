use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

const CINEMETA: &str = "https://v3-cinemeta.strem.io";
const WIKIDATA: &str = "https://query.wikidata.org/sparql";
type WikiLinks = std::collections::HashMap<String, (String, Option<String>, Option<String>)>;

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

fn decode_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let code = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            output.push(u8::from_str_radix(code, 16).ok()?);
            index += 3;
        } else {
            output.push(if bytes[index] == b'_' {
                b' '
            } else {
                bytes[index]
            });
            index += 1;
        }
    }
    String::from_utf8(output).ok()
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
        .filter_map(|item| {
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
        })
        .collect()
}

fn russian_links(agent: &ureq::Agent, ids: &[&str]) -> Result<WikiLinks, String> {
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
    let query = format!("SELECT ?id ?label ?article ?enarticle WHERE {{ VALUES ?id {{ {values} }} ?item <http://www.wikidata.org/prop/direct/P345> ?id. OPTIONAL {{ ?item <http://www.w3.org/2000/01/rdf-schema#label> ?label. FILTER(LANG(?label)=\"ru\") }} OPTIONAL {{ ?article <http://schema.org/about> ?item; <http://schema.org/isPartOf> <https://ru.wikipedia.org/>. }} OPTIONAL {{ ?enarticle <http://schema.org/about> ?item; <http://schema.org/isPartOf> <https://en.wikipedia.org/>. }} }}");
    let payload = json(
        agent,
        &format!("{WIKIDATA}?format=json&query={}", encode(&query)),
    )?;
    Ok(parse_wikidata_links(&payload))
}

fn parse_wikidata_links(payload: &Value) -> WikiLinks {
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
            let article = row
                .pointer("/article/value")
                .and_then(Value::as_str)
                .and_then(|url| url.strip_prefix("https://ru.wikipedia.org/wiki/"))
                .and_then(decode_path);
            let english_article = row
                .pointer("/enarticle/value")
                .and_then(Value::as_str)
                .and_then(|url| url.strip_prefix("https://en.wikipedia.org/wiki/"))
                .and_then(decode_path);
            matches.insert(
                id.to_owned(),
                (label.trim().to_owned(), article, english_article),
            );
        }
    }
    matches
}

pub fn popular() -> Result<Vec<Movie>, String> {
    let agent = agent_with_timeout(Duration::from_secs(5));
    let mut items = parse_catalog(&json(
        &agent,
        &format!("{CINEMETA}/catalog/movie/top.json"),
    )?);
    if items.is_empty() {
        return Err("Cinemeta вернула пустой каталог".into());
    }
    let ids = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    if let Ok(names) = russian_links(&agent, &ids) {
        let articles = names
            .values()
            .filter_map(|(_, article, _)| article.as_deref())
            .collect::<Vec<_>>();
        let english_articles = names
            .values()
            .filter_map(|(_, _, article)| article.as_deref())
            .collect::<Vec<_>>();
        let thumbnails = wikipedia_thumbnails(&agent, "ru", &articles).unwrap_or_default();
        let english_thumbnails =
            wikipedia_thumbnails(&agent, "en", &english_articles).unwrap_or_default();
        for item in &mut items {
            if let Some((name, article, english_article)) = names.get(&item.id) {
                if !name.is_empty() {
                    item.title = name.clone();
                }
                item.alternate_poster_url = article
                    .as_ref()
                    .and_then(|title| thumbnails.get(title))
                    .or_else(|| {
                        english_article
                            .as_ref()
                            .and_then(|title| english_thumbnails.get(title))
                    })
                    .cloned();
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
        poster_url: Some(format!(
            "https://images.metahub.space/poster/medium/{id}/img"
        )),
        alternate_poster_url: None,
        background_url: None,
        overview: None,
        genres: Vec::new(),
    })
    .collect()
}

fn wikipedia_thumbnails(
    agent: &ureq::Agent,
    language: &str,
    titles: &[&str],
) -> Result<std::collections::HashMap<String, String>, String> {
    if !matches!(language, "ru" | "en") {
        return Err("Неподдерживаемый язык Wikipedia".into());
    }
    let titles = titles.iter().copied().take(30).collect::<Vec<_>>();
    if titles.is_empty() {
        return Ok(Default::default());
    }
    let url = format!("https://{language}.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&prop=pageimages&pithumbsize=600&titles={}", encode(&titles.join("|")));
    let payload = json(agent, &url)?;
    Ok(payload
        .pointer("/query/pages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|page| {
            Some((
                page.get("title")?.as_str()?.to_owned(),
                page.pointer("/thumbnail/source")?.as_str()?.to_owned(),
            ))
        })
        .collect())
}

fn wikipedia_imdb_id(agent: &ureq::Agent, title: &str) -> Option<String> {
    let page = json(
        agent,
        &format!(
            "https://ru.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&prop=pageprops&ppprop=wikibase_item&titles={}",
            encode(title)
        ),
    )
    .ok()?;
    let item = page
        .pointer("/query/pages/0/pageprops/wikibase_item")
        .and_then(Value::as_str)?;
    let entity = json(
        agent,
        &format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&format=json&props=claims&ids={}",
            encode(item)
        ),
    )
    .ok()?;
    let imdb = entity
        .pointer(&format!(
            "/entities/{item}/claims/P345/0/mainsnak/datavalue/value"
        ))
        .and_then(Value::as_str)?;
    valid_imdb(imdb).then(|| imdb.to_owned())
}

pub fn cached_popular(cache: &Path) -> Vec<Movie> {
    let Ok(bytes) = std::fs::read(cache.join("popular.json")) else {
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
    std::fs::create_dir_all(cache).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(items).map_err(|error| error.to_string())?;
    std::fs::write(cache.join("popular.json"), bytes).map_err(|error| error.to_string())
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

fn russian_wikipedia(
    agent: &ureq::Agent,
    title: &str,
    wanted_year: Option<i64>,
    series: bool,
) -> Option<Movie> {
    if !title
        .chars()
        .any(|ch| ('А'..='я').contains(&ch) || ch == 'ё' || ch == 'Ё')
    {
        return None;
    }
    let topic = if series { "сериал" } else { "фильм" };
    let url = format!("https://ru.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&generator=search&gsrnamespace=0&gsrlimit=8&gsrsearch={}&prop=pageimages%7Cextracts&pithumbsize=600&exintro=1&explaintext=1", encode(&format!("{title} {topic}")));
    let payload = json(agent, &url).ok()?;
    payload
        .pointer("/query/pages")?
        .as_array()?
        .iter()
        .find_map(|page| {
            let page_title = page.get("title")?.as_str()?;
            let clean_page_title = page_title.split(" (").next().unwrap_or(page_title);
            if normalized(clean_page_title) != normalized(title) {
                return None;
            }
            let overview = page.get("extract")?.as_str()?.trim();
            let intro = overview.lines().next().unwrap_or(overview).to_lowercase();
            let is_series = intro.contains("сериал") || intro.contains("телевизионн");
            let is_movie = intro.contains("фильм")
                || intro.contains("мультфильм")
                || intro.contains("кинокартин");
            if series != is_series || !is_series && !is_movie {
                return None;
            }
            let found_year =
                year(&Value::String(page_title.to_owned())).or_else(|| year(&Value::String(intro)));
            if wanted_year.is_some_and(|expected| {
                found_year.is_some_and(|found| (found - expected).abs() > 1)
            }) && !series
            {
                return None;
            }
            let id = wikipedia_imdb_id(agent, page_title).unwrap_or_else(|| {
                format!(
                    "wiki:{}",
                    page.get("pageid")
                        .and_then(Value::as_i64)
                        .unwrap_or_default()
                )
            });
            Some(Movie {
                id,
                title: clean_page_title.to_owned(),
                original_title: title.to_owned(),
                year: found_year.or(wanted_year),
                rating: None,
                poster_url: page
                    .pointer("/thumbnail/source")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                alternate_poster_url: None,
                background_url: None,
                overview: Some(overview.to_owned()),
                genres: Vec::new(),
            })
        })
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
    let wiki = russian_wikipedia(&agent, &clean, wanted_year, series)
        .or_else(|| russian_wikipedia(&agent, &clean, wanted_year, !series));
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
        if wiki.is_some() {
            return Ok(wiki);
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
    if let Some(wiki) = wiki {
        item.title = wiki.title;
        item.overview = wiki.overview;
        item.alternate_poster_url = wiki.poster_url;
        return Ok(Some(item));
    }
    if let Ok(names) = russian_links(&agent, &[&item.id]) {
        if let Some((name, article, english_article)) = names.get(&item.id) {
            if !name.is_empty() {
                item.title = name.clone();
            }
            if let Some(article) = article {
                item.alternate_poster_url = wikipedia_thumbnails(&agent, "ru", &[article])
                    .ok()
                    .and_then(|images| images.get(article).cloned());
                let url = format!("https://ru.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&prop=extracts&exintro=1&explaintext=1&titles={}", encode(article));
                if let Ok(payload) = json(&agent, &url) {
                    if let Some(extract) = payload
                        .pointer("/query/pages/0/extract")
                        .and_then(Value::as_str)
                    {
                        if !extract.trim().is_empty() {
                            item.overview = Some(extract.trim().to_owned());
                        }
                    }
                }
            }
            if item.alternate_poster_url.is_none() {
                if let Some(article) = english_article {
                    item.alternate_poster_url = wikipedia_thumbnails(&agent, "en", &[article])
                        .ok()
                        .and_then(|images| images.get(article).cloned());
                }
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

pub fn movie_poster_jpeg(movie: &Movie, series: bool) -> Result<Vec<u8>, String> {
    let mut urls = Vec::new();
    if let Some(primary) = movie.poster_url.as_deref() {
        urls.push(primary.to_owned());
        if primary.contains("images.metahub.space/poster/small/") {
            urls.push(primary.replacen("/poster/small/", "/poster/medium/", 1));
        }
    }
    if let Some(alternate) = movie.alternate_poster_url.as_deref() {
        if !urls.iter().any(|url| url == alternate) {
            urls.push(alternate.to_owned());
        }
    }
    if let Some(background) = movie.background_url.as_deref() {
        if !urls.iter().any(|url| url == background) {
            urls.push(background.to_owned());
        }
    }
    if valid_imdb(&movie.id) {
        urls.push(format!(
            "https://images.metahub.space/poster/medium/{}/img",
            movie.id
        ));
    }
    for url in urls {
        if let Ok(bytes) = poster_jpeg(&url) {
            return Ok(bytes);
        }
    }
    if valid_imdb(&movie.id) {
        let agent = agent();
        if let Ok(links) = russian_links(&agent, &[&movie.id]) {
            if let Some((_, russian, english)) = links.get(&movie.id) {
                for (language, article) in [("ru", russian), ("en", english)] {
                    if let Some(article) = article {
                        if let Some(url) = wikipedia_thumbnails(&agent, language, &[article])
                            .ok()
                            .and_then(|images| images.get(article).cloned())
                        {
                            if let Ok(bytes) = poster_jpeg(&url) {
                                return Ok(bytes);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(wiki) = russian_wikipedia(&agent(), &movie.title, movie.year, series) {
        if let Some(url) = wiki.poster_url {
            return poster_jpeg(&url);
        }
    }
    Err("Не удалось получить обложку из Cinemeta или Wikipedia".into())
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
    fn wikipedia_article_path_is_decoded_once() {
        assert_eq!(
            decode_path("%D0%94%D1%8E%D0%BD%D0%B0_(%D1%84%D0%B8%D0%BB%D1%8C%D0%BC)"),
            Some("Дюна (фильм)".into())
        );
    }

    #[test]
    fn wikidata_links_keep_english_wikipedia_when_russian_poster_is_missing() {
        let payload = serde_json::json!({"results":{"bindings":[{
            "id":{"value":"tt123"},
            "label":{"value":"Фильм"},
            "enarticle":{"value":"https://en.wikipedia.org/wiki/Test_film"}
        }]}});
        let links = parse_wikidata_links(&payload);
        assert_eq!(
            links.get("tt123"),
            Some(&("Фильм".into(), None, Some("Test film".into())))
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
    #[ignore = "uses public Cinemeta and Wikipedia"]
    fn live_series_bundle_gets_metadata_and_poster() {
        let movie = lookup(
            "Мажор [S01-05 + Мажор. Фильм + Мажор в Сочи] (2014-2025) WEB-DL",
            true,
        )
        .unwrap()
        .expect("Мажор must resolve through a public source");
        assert!(movie.overview.is_some());
        assert!(!movie_poster_jpeg(&movie, true).unwrap().is_empty());
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
        let mut jpeg = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(1, 1)
            .write_to(&mut jpeg, image::ImageFormat::Jpeg)
            .unwrap();
        cache_poster(&cache, "tt123", jpeg.get_ref()).unwrap();
        assert_eq!(cached_poster(&cache, "tt123"), Some(jpeg.into_inner()));
        std::fs::write(cache.join("popular.json"), b"{").unwrap();
        assert!(cached_popular(&cache).is_empty());
        std::fs::remove_file(cache.join("popular.json")).unwrap();
        std::fs::remove_file(cache.join("tt123.jpg")).unwrap();
        std::fs::remove_dir(cache).unwrap();
    }

    #[test]
    fn offline_home_catalog_has_valid_russian_cards() {
        let items = fallback_popular();
        assert!(items.len() >= 12);
        assert!(items.iter().all(|item| valid_imdb(&item.id)));
        assert!(items.iter().all(|item| item
            .poster_url
            .as_deref()
            .is_some_and(|url| url.starts_with("https://"))));
        assert!(items
            .iter()
            .any(|item| item.title.chars().any(|ch| ('А'..='я').contains(&ch))));
    }
}
