//! Deterministic, local-first cover art.
//!
//! Posters downloaded from metadata providers are cached as JPEG files. Some
//! releases never get a poster, and many provider posters are noisy or
//! mismatched. This module builds a clean, typographic title card from the
//! metadata that is already stored on the machine. Nothing here performs I/O
//! or touches the network: the same hash and metadata always produce the same
//! SVG, so the result is stable across restarts and backups.

use base64::Engine as _;

#[derive(Clone, Copy, Debug)]
pub struct CoverRequest<'a> {
    pub hash: &'a str,
    pub raw_title: &'a str,
    pub title: Option<&'a str>,
    pub year: Option<i64>,
    pub rating: Option<f64>,
    pub genres: &'a [String],
    pub media_type: Option<&'a str>,
}

struct Palette {
    base: &'static str,
    deep: &'static str,
    accent: &'static str,
}

// Muted, cinema-grade dark grounds. The accent only appears in the thin rules,
// the small kicker and the spotlight; the title itself stays ivory.
const PALETTES: [Palette; 6] = [
    Palette {
        base: "#0d0e11",
        deep: "#241b12",
        accent: "#d9a45f",
    },
    Palette {
        base: "#100d0e",
        deep: "#2a1518",
        accent: "#cf8d7c",
    },
    Palette {
        base: "#0c100e",
        deep: "#182820",
        accent: "#a9bd94",
    },
    Palette {
        base: "#0c1014",
        deep: "#182431",
        accent: "#9db3c9",
    },
    Palette {
        base: "#110d13",
        deep: "#27182b",
        accent: "#c09bc2",
    },
    Palette {
        base: "#0b1112",
        deep: "#102628",
        accent: "#8fbab5",
    },
];

pub fn data_uri(request: &CoverRequest<'_>) -> String {
    let svg = render_svg(request);
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg.as_bytes())
    )
}

pub fn render_svg(request: &CoverRequest<'_>) -> String {
    let (cleaned, detected_year) = clean_release_title(request.raw_title);
    let title = request
        .title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(display_title)
        .filter(|title| !title.is_empty())
        .unwrap_or(cleaned);
    let title = if title.is_empty() {
        "Untitled".to_owned()
    } else {
        title
    };
    let year = request.year.or(detected_year);
    let series = request
        .media_type
        .is_some_and(|kind| kind.eq_ignore_ascii_case("series"))
        || (request.media_type.is_none() && looks_like_series(request.raw_title));
    let hash = fnv1a(request.hash.as_bytes());
    let palette = &PALETTES[(hash as usize) % PALETTES.len()];
    let angle = 8 + (hash >> 3) % 22;
    let glow_x = 320 + (hash >> 8) % 230;
    let glow_y = 120 + (hash >> 16) % 150;
    let monogram = title
        .chars()
        .find(|character| character.is_alphanumeric())
        .map(|character| character.to_uppercase().collect::<String>())
        .unwrap_or_default();
    let kind_label = if series { "SERIES" } else { "FILM" };
    let kicker = match year {
        Some(year) => format!("{kind_label} · {year}"),
        None => kind_label.to_owned(),
    };
    let genres = request
        .genres
        .iter()
        .map(|genre| genre.trim())
        .filter(|genre| !genre.is_empty())
        .take(2)
        .map(collapse_spaces)
        .collect::<Vec<_>>()
        .join(" · ");
    let rating = request
        .rating
        .filter(|rating| (0.0..=10.0).contains(rating))
        .map(|rating| format!("★ {rating:.1}"))
        .unwrap_or_default();
    let (font_size, line_height, max_chars) = title_scale(&title);
    let lines = wrap_title(&title, max_chars, 3);
    let title_top = 320.0 - (lines.len().saturating_sub(1) as f32 * line_height / 2.0);
    let mut title_svg = String::new();
    for (index, line) in lines.iter().enumerate() {
        title_svg.push_str(&format!(
            "<text x=\"58\" y=\"{:.1}\" class=\"title\">{}</text>",
            title_top + index as f32 * line_height,
            escape(line)
        ));
    }
    let monogram = if monogram.is_empty() {
        String::new()
    } else {
        format!(
            "<text x=\"552\" y=\"792\" text-anchor=\"end\" class=\"monogram\">{}</text>",
            escape(&monogram)
        )
    };
    let genre_line = if genres.is_empty() {
        String::new()
    } else {
        format!(
            "<text x=\"58\" y=\"790\" class=\"meta\">{}</text>",
            escape(&genres)
        )
    };
    let rating_badge = if rating.is_empty() {
        String::new()
    } else {
        format!(
            "<text x=\"542\" y=\"88\" text-anchor=\"end\" class=\"rating\">{}</text>",
            escape(&rating)
        )
    };
    let year_line = year.map(|year| year.to_string()).unwrap_or_default();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="900" viewBox="0 0 600 900" role="img" aria-label="{aria}">
<defs>
<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1" gradientTransform="rotate({angle} .5 .5)">
<stop offset="0" stop-color="{base}"/>
<stop offset="0.56" stop-color="{deep}"/>
<stop offset="1" stop-color="#07080a"/>
</linearGradient>
<radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
<stop offset="0" stop-color="{accent}" stop-opacity="0.34"/>
<stop offset="0.55" stop-color="{accent}" stop-opacity="0.10"/>
<stop offset="1" stop-color="{accent}" stop-opacity="0"/>
</radialGradient>
<linearGradient id="rule" x1="0" y1="0" x2="1" y2="0">
<stop offset="0" stop-color="{accent}" stop-opacity="0.85"/>
<stop offset="0.55" stop-color="{accent}" stop-opacity="0.18"/>
<stop offset="1" stop-color="{accent}" stop-opacity="0"/>
</linearGradient>
<linearGradient id="shade" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#07080a" stop-opacity="0"/>
<stop offset="0.7" stop-color="#07080a" stop-opacity="0.28"/>
<stop offset="1" stop-color="#07080a" stop-opacity="0.9"/>
</linearGradient>
<style>
.title {{ fill: #f4f0e7; font-family: Georgia, "Palatino Linotype", "Times New Roman", serif; font-size: {font_size}px; font-weight: 600; letter-spacing: -1.5px; }}
.monogram {{ fill: #f4f0e7; opacity: 0.055; font-family: Georgia, "Palatino Linotype", serif; font-size: 390px; font-weight: 700; }}
.meta {{ fill: #d8d2c6; opacity: 0.8; font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; font-size: 18px; letter-spacing: 1.6px; text-transform: uppercase; }}
.kicker {{ fill: {accent}; font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; font-size: 15px; font-weight: 600; letter-spacing: 3.4px; }}
.rating {{ fill: #f0d6a9; font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; font-size: 18px; font-weight: 600; letter-spacing: 0.6px; }}
.brand {{ fill: #f4f0e7; opacity: 0.55; font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; font-size: 12px; font-weight: 600; letter-spacing: 3px; }}
.serial {{ fill: #f4f0e7; opacity: 0.28; font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; font-size: 11px; letter-spacing: 1.5px; }}
</style>
</defs>
<rect width="600" height="900" fill="url(#bg)"/>
<ellipse cx="{glow_x}" cy="{glow_y}" rx="360" ry="300" fill="url(#glow)"/>
<path d="M0 690 C 150 612, 390 742, 600 648 L600 900 L0 900 Z" fill="{accent}" opacity="0.045"/>
<path d="M410 0 L600 0 L600 245 C 506 214, 458 126, 410 0 Z" fill="{accent}" opacity="0.05"/>
<g opacity="0.22" stroke="{accent}" stroke-width="1">
<line x1="58" y1="132" x2="542" y2="132"/>
</g>
<rect x="58" y="82" width="8" height="8" fill="{accent}"/>
<text x="78" y="88" class="kicker">{kicker}</text>
{rating_badge}
<rect x="0" y="650" width="600" height="250" fill="url(#shade)"/>
{monogram}
<line x1="58" y1="684" x2="188" y2="684" stroke="url(#rule)" stroke-width="2"/>
{title_svg}
<g opacity="0.8">
<line x1="58" y1="752" x2="542" y2="752" stroke="{accent}" stroke-opacity="0.24" stroke-width="1"/>
</g>
{genre_line}
<text x="58" y="838" class="brand">PIRATE CINEMA</text>
<text x="542" y="838" text-anchor="end" class="serial">LOCAL COVER · {year_line}</text>
</svg>"##,
        aria = escape(&format!("{title} local cover")),
        angle = angle,
        base = palette.base,
        deep = palette.deep,
        accent = palette.accent,
        font_size = font_size,
        glow_x = glow_x,
        glow_y = glow_y,
        kicker = escape(&kicker),
        rating_badge = rating_badge,
        monogram = monogram,
        title_svg = title_svg,
        genre_line = genre_line,
        year_line = escape(&year_line),
    )
}

/// Keep a title that is already readable, but clean imported metadata that
/// still carries a release name. Some old profiles stored the torrent title as
/// the metadata title, so `"Мажор (телесериал)"` must stay intact while
/// `"Мажор [S01-05] (2014) WEB-DL"` becomes `"Мажор"`.
pub fn display_title(value: &str) -> String {
    let collapsed = collapse_spaces(value);
    if collapsed.is_empty() {
        return collapsed;
    }
    let looks_like_release = collapsed.split_whitespace().any(|word| {
        let token = word.trim_matches(|character: char| {
            matches!(character, '-' | '–' | '—' | ':' | ';' | ',' | '\'' | '"')
        });
        !token.is_empty() && (is_episode_marker(token) || is_release_token(token))
    });
    if looks_like_release {
        let (cleaned, _) = clean_release_title(&collapsed);
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    collapsed
}

/// Turn a torrent release name into a readable title and the year, when one is
/// present. Metadata titles are not passed through this function because they
/// are already clean and may legitimately contain words that look like tags.
pub fn clean_release_title(raw: &str) -> (String, Option<i64>) {
    let year = plausible_year(raw);
    let without_brackets = strip_bracketed(raw);
    let mut normalized = String::with_capacity(without_brackets.len());
    for character in without_brackets.chars() {
        if matches!(character, '.' | '_' | '+' | '|' | '/' | '\\') {
            normalized.push(' ');
        } else {
            normalized.push(character);
        }
    }
    let normalized = collapse_spaces(&normalized);
    let mut words = Vec::new();
    let mut previous_was_tag = false;
    for word in normalized.split_whitespace() {
        let trimmed = word.trim_matches(|character: char| {
            matches!(character, '-' | '–' | '—' | ';' | ',' | '\'' | '"')
        });
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if matches!(lower.as_str(), "web" | "blu" | "ray") && previous_was_tag {
            previous_was_tag = false;
            continue;
        }
        if is_episode_marker(trimmed) {
            previous_was_tag = false;
            continue;
        }
        if is_release_token(trimmed) {
            previous_was_tag = is_release_tag(trimmed);
            continue;
        }
        words.push(trimmed.to_owned());
        previous_was_tag = false;
    }
    if let Some(year) = year {
        if let Some(index) = words.iter().position(|word| word == &year.to_string()) {
            if index >= 1 {
                words.truncate(index);
            }
        }
    }
    let title = collapse_spaces(&words.join(" "));
    let title = title
        .trim_matches(|character: char| matches!(character, '-' | '–' | '—' | ':' | ';' | ','))
        .to_owned();
    if title.is_empty() {
        (collapse_spaces(raw), year)
    } else {
        (title, year)
    }
}

fn plausible_year(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let mut found = None;
    for window in bytes.windows(4) {
        if window.iter().all(u8::is_ascii_digit) {
            if let Ok(year) = std::str::from_utf8(window)
                .unwrap_or_default()
                .parse::<i64>()
            {
                if (1900..=2035).contains(&year) {
                    found = Some(year);
                }
            }
        }
    }
    found
}

fn strip_bracketed(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut depth = 0_u32;
    for character in value.chars() {
        match character {
            '[' | '(' | '{' => depth = depth.saturating_add(1),
            ']' | ')' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => result.push(character),
            _ => {}
        }
    }
    result
}

fn is_episode_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.len() >= 3
        && lower.starts_with('s')
        && lower[1..].chars().take_while(char::is_ascii_digit).count() >= 1
        && lower.contains('e')
    {
        return true;
    }
    if lower.len() >= 2
        && lower.starts_with('e')
        && lower[1..]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return true;
    }
    if let Some((season, episode)) = lower.split_once('x') {
        return season.chars().all(|character| character.is_ascii_digit())
            && !season.is_empty()
            && episode.chars().all(|character| character.is_ascii_digit())
            && !episode.is_empty();
    }
    false
}

fn looks_like_series(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(is_episode_marker)
}

fn is_release_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if matches!(lower.as_str(), "web" | "ray" | "blu") {
        return false;
    }
    if is_release_tag(value) {
        return true;
    }
    // Release groups are usually glued to the last tag (`h264-NTb`,
    // `WEB-DL-GROUP`). Dropping the whole compound removes the group without
    // guessing at standalone words.
    lower
        .split_once('-')
        .is_some_and(|(prefix, _)| is_release_tag(prefix) || matches!(prefix, "web" | "blu"))
}

fn is_release_tag(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "2160p"
            | "1440p"
            | "1080p"
            | "1080i"
            | "720p"
            | "576p"
            | "480p"
            | "360p"
            | "4k"
            | "8k"
            | "uhd"
            | "bluray"
            | "bdrip"
            | "brrip"
            | "webrip"
            | "webdl"
            | "hdtv"
            | "dvdrip"
            | "dvd"
            | "remux"
            | "x264"
            | "x265"
            | "h264"
            | "h265"
            | "hevc"
            | "avc"
            | "aac"
            | "ac3"
            | "eac3"
            | "ddp"
            | "dd"
            | "dts"
            | "truehd"
            | "atmos"
            | "flac"
            | "opus"
            | "mp3"
            | "hdr"
            | "hdr10"
            | "hdr10+"
            | "dv"
            | "dolby"
            | "vision"
            | "proper"
            | "repack"
            | "internal"
            | "limited"
            | "complete"
            | "readnfo"
            | "10bit"
            | "8bit"
            | "multi"
            | "dubbed"
            | "subbed"
            | "sub"
            | "subs"
            | "rus"
            | "eng"
            | "ukr"
            | "ita"
            | "ger"
            | "fre"
            | "spa"
            | "amzn"
            | "nf"
            | "dsnp"
            | "hmax"
            | "atvp"
            | "extended"
            | "uncut"
            | "remastered"
    )
}

fn title_scale(title: &str) -> (u32, f32, usize) {
    let count = title.chars().count();
    if count <= 11 {
        (76, 78.0, 10)
    } else if count <= 21 {
        (64, 68.0, 13)
    } else if count <= 34 {
        (54, 58.0, 16)
    } else {
        (44, 49.0, 20)
    }
}

fn wrap_title(title: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in title.split_whitespace() {
        let word_chars = word.chars().count();
        let current_chars = current.chars().count();
        if !current.is_empty() && current_chars + 1 + word_chars > max_chars {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                break;
            }
        }
        if word_chars > max_chars {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                if lines.len() == max_lines {
                    break;
                }
            }
            let mut remainder = String::new();
            for character in word.chars() {
                if remainder.chars().count() == max_chars {
                    lines.push(std::mem::take(&mut remainder));
                    if lines.len() == max_lines {
                        break;
                    }
                }
                remainder.push(character);
            }
            if !remainder.is_empty() {
                current = remainder;
            }
            if lines.len() == max_lines {
                break;
            }
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(title.to_owned());
    }
    if lines.len() == max_lines {
        let consumed = lines.iter().map(|line| line.chars().count()).sum::<usize>()
            + lines.len().saturating_sub(1);
        if consumed < title.chars().count() {
            if let Some(last) = lines.last_mut() {
                while last.chars().count() + 1 > max_chars {
                    last.pop();
                }
                last.push('…');
            }
        }
    }
    lines
}

fn collapse_spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\u{0000}'..='\u{001f}' => escaped.push(' '),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_names_become_clean_titles() {
        assert_eq!(
            clean_release_title("Dune.Part.Two.2024.2160p.WEB-DL.DDP5.1.Atmos.H.265-FLUX"),
            ("Dune Part Two".to_owned(), Some(2024))
        );
        assert_eq!(
            clean_release_title("Severance.S02E07.1080p.WEB.h264-NTb"),
            ("Severance".to_owned(), None)
        );
        assert_eq!(
            clean_release_title("Blade Runner 2049 (2017) [1080p]"),
            ("Blade Runner 2049".to_owned(), Some(2017))
        );
    }

    #[test]
    fn imported_release_titles_are_cleaned_but_clean_titles_are_kept() {
        assert_eq!(
            display_title("Мажор [S01-05 + Мажор. Фильм] (2014-2025) WEB-DL, WEBRip-ExKinoRay"),
            "Мажор"
        );
        assert_eq!(
            display_title("Неуловимые мстители: Трилогия (1966-1971) BDRip 1080p"),
            "Неуловимые мстители: Трилогия"
        );
        assert_eq!(display_title("Мажор (телесериал)"), "Мажор (телесериал)");
        assert_eq!(display_title("The Web of Tomorrow"), "The Web of Tomorrow");
    }

    #[test]
    fn metadata_titles_are_not_stripped() {
        let hash = "a".repeat(40);
        let request = CoverRequest {
            hash: &hash,
            raw_title: "Movie.2022.1080p.WEB-DL.x264-GROUP",
            title: Some("The Web of Tomorrow"),
            year: Some(2022),
            rating: Some(8.1),
            genres: &["Drama".to_owned()],
            media_type: Some("movie"),
        };
        let svg = render_svg(&request);
        assert!(svg.contains("The Web of Tomorrow"));
        assert!(svg.contains("★ 8.1"));
        assert!(svg.contains("Drama"));
        assert!(!svg.contains("Movie.2022"));
    }

    #[test]
    fn generated_cover_is_deterministic_and_escaped() {
        let hash = "b".repeat(40);
        let genres = ["Drama & Crime".to_owned()];
        let request = CoverRequest {
            hash: &hash,
            raw_title: "A.Title.2024.1080p",
            title: Some("A <Title> & \"Friends\""),
            year: Some(2024),
            rating: None,
            genres: &genres,
            media_type: Some("movie"),
        };
        let first = data_uri(&request);
        let second = data_uri(&request);
        assert_eq!(first, second);
        assert!(first.starts_with("data:image/svg+xml;base64,"));
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(first.trim_start_matches("data:image/svg+xml;base64,"))
            .unwrap();
        let svg = String::from_utf8(decoded).unwrap();
        assert!(svg.contains("A &lt;Title&gt; &amp; &quot;Friends&quot;"));
        assert!(!svg.contains("<Title>"));
    }

    #[test]
    fn wrapping_respects_line_and_character_limits() {
        let lines = wrap_title("The Curious Case of Benjamin Button Returns Again", 16, 3);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| line.chars().count() <= 16));
        assert!(lines.last().is_some_and(|line| line.ends_with('…')));
    }

    #[test]
    fn empty_and_unicode_titles_still_render() {
        let request = CoverRequest {
            hash: "",
            raw_title: "",
            title: None,
            year: None,
            rating: Some(11.0),
            genres: &[],
            media_type: None,
        };
        let svg = render_svg(&request);
        assert!(svg.contains("Untitled"));
        assert!(!svg.contains("★ 11.0"));

        let unicode = CoverRequest {
            hash: "hash",
            raw_title: "Мастер и Маргарита 2024 1080p",
            title: None,
            year: None,
            rating: None,
            genres: &[],
            media_type: Some("movie"),
        };
        assert!(render_svg(&unicode).contains("Мастер и Маргарита"));
    }
}
