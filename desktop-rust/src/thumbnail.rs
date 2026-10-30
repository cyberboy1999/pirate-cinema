use image::{DynamicImage, GenericImageView, ImageFormat};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const SAMPLE_SECONDS: [u64; 5] = [1, 10, 90, 600, 1_800];
const MAX_FRAME_BYTES: u64 = 12 * 1024 * 1024;
const FRAME_TIMEOUT: Duration = Duration::from_secs(24);
const GENERATION_TIMEOUT: Duration = Duration::from_secs(75);
static CACHE_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct GeneratedThumbnail {
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameScore {
    pub brightness: f64,
    pub contrast: f64,
    pub sharpness: f64,
    pub total: f64,
}

pub struct FrameAnalyzer;

impl FrameAnalyzer {
    pub fn score(image: &DynamicImage) -> Option<FrameScore> {
        let gray = image.thumbnail(320, 180).to_luma8();
        let count = f64::from(gray.width()) * f64::from(gray.height());
        if count < 4.0 {
            return None;
        }
        let brightness = gray.pixels().map(|pixel| f64::from(pixel[0])).sum::<f64>() / count;
        let contrast = (gray
            .pixels()
            .map(|pixel| {
                let delta = f64::from(pixel[0]) - brightness;
                delta * delta
            })
            .sum::<f64>()
            / count)
            .sqrt();
        let mut edges = 0.0;
        let mut edge_count = 0_u64;
        for y in 1..gray.height() {
            for x in 1..gray.width() {
                let here = f64::from(gray.get_pixel(x, y)[0]);
                edges += (here - f64::from(gray.get_pixel(x - 1, y)[0])).abs();
                edges += (here - f64::from(gray.get_pixel(x, y - 1)[0])).abs();
                edge_count += 2;
            }
        }
        let sharpness = edges / edge_count.max(1) as f64;
        if !(22.0..=238.0).contains(&brightness) || contrast < 18.0 || sharpness < 1.5 {
            return None;
        }
        let exposure = 1.0 - ((brightness - 128.0).abs() / 128.0).min(1.0);
        Some(FrameScore {
            brightness,
            contrast,
            sharpness,
            total: contrast * 0.45 + sharpness * 1.8 + exposure * 24.0,
        })
    }
}

pub struct ThumbnailSelector;

impl ThumbnailSelector {
    pub fn best(frames: Vec<(DynamicImage, FrameScore)>) -> Option<(DynamicImage, FrameScore)> {
        frames.into_iter().max_by(|left, right| {
            left.1
                .total
                .partial_cmp(&right.1.total)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

#[derive(Clone)]
pub struct ThumbnailCache {
    root: PathBuf,
}

impl ThumbnailCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn store(&self, key: &str, bytes: &[u8]) -> Result<String, String> {
        let key = safe_key(key)?;
        std::fs::create_dir_all(&self.root).map_err(|error| error.to_string())?;
        let generation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let sequence = CACHE_GENERATION.fetch_add(1, Ordering::Relaxed);
        let name = format!("generated-{key}-{generation:x}-{sequence:x}.webp");
        let temporary = self.root.join(format!(".{name}.part"));
        std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        std::fs::rename(&temporary, self.root.join(&name)).map_err(|error| error.to_string())?;
        Ok(name)
    }

    pub fn path(&self, name: &str) -> Option<PathBuf> {
        (name.starts_with("generated-") && name.ends_with(".webp") && !name.contains(['/', '\\']))
            .then(|| self.root.join(name))
    }
}

pub struct ThumbnailResolver;

impl ThumbnailResolver {
    pub fn selected<'a>(
        generated: Option<(&'a str, bool)>,
        original: Option<&'a str>,
    ) -> Option<&'a str> {
        generated
            .filter(|(_, applied)| *applied)
            .map(|(name, _)| name)
            .or(original)
    }
}

pub struct ThumbnailEngine {
    ffmpeg: PathBuf,
    cache: ThumbnailCache,
}

impl ThumbnailEngine {
    pub fn new(ffmpeg: PathBuf, cache: ThumbnailCache) -> Self {
        Self { ffmpeg, cache }
    }

    pub fn generate(
        &self,
        source: &str,
        key: &str,
        cancelled: Arc<AtomicBool>,
    ) -> Result<GeneratedThumbnail, String> {
        let local_stream =
            source.starts_with("http://127.0.0.1:") || source.starts_with("http://[::1]:");
        if !local_stream && !Path::new(source).is_file() {
            return Err(
                "Видеоисточник недоступен или не является локальным потоком приложения".into(),
            );
        }
        let temporary = std::env::temp_dir().join(format!(
            "pirate-cinema-thumbnail-{}-{}",
            std::process::id(),
            safe_key(key)?
        ));
        std::fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
        let result = self.generate_inner(source, key, &temporary, &cancelled);
        let _ = std::fs::remove_dir_all(temporary);
        result
    }

    fn generate_inner(
        &self,
        source: &str,
        key: &str,
        temporary: &Path,
        cancelled: &AtomicBool,
    ) -> Result<GeneratedThumbnail, String> {
        let mut candidates = Vec::new();
        let generation_started = Instant::now();
        for (index, seconds) in SAMPLE_SECONDS.into_iter().enumerate() {
            ensure_running(cancelled)?;
            if generation_started.elapsed() > GENERATION_TIMEOUT {
                break;
            }
            let output = temporary.join(format!("frame-{index}.png"));
            if extract_frame(&self.ffmpeg, source, seconds, &output, cancelled).is_err() {
                continue;
            }
            let Ok(metadata) = std::fs::metadata(&output) else {
                continue;
            };
            if metadata.len() == 0 || metadata.len() > MAX_FRAME_BYTES {
                continue;
            }
            let Ok(bytes) = std::fs::read(&output) else {
                continue;
            };
            let Ok(image) = image::load_from_memory(&bytes) else {
                continue;
            };
            if let Some(score) = FrameAnalyzer::score(&image) {
                candidates.push((image, score));
            }
        }
        ensure_running(cancelled)?;
        let (image, score) = ThumbnailSelector::best(candidates)
            .ok_or("FFmpeg не нашёл достаточно качественный кадр")?;
        let cover = cover_crop(image, 600, 900);
        let mut bytes = Vec::new();
        cover
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)
            .map_err(|error| format!("Не удалось сформировать WebP: {error}"))?;
        if bytes.len() > 5 * 1024 * 1024 {
            return Err("Сгенерированная обложка превышает 5 МиБ".into());
        }
        let file_name = self.cache.store(key, &bytes)?;
        Ok(GeneratedThumbnail {
            file_name,
            bytes,
            score: score.total,
        })
    }
}

fn extract_frame(
    ffmpeg: &Path,
    source: &str,
    seconds: u64,
    output: &Path,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let mut command = Command::new(ffmpeg);
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-rw_timeout",
            "10000000",
        ])
        .args(["-ss", &seconds.to_string(), "-i", source])
        .args(["-frames:v", "1", "-vf", "scale=960:-2", "-y"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("FFmpeg: {error}"))?;
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Генерация отменена".into());
        }
        if started.elapsed() > FRAME_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Извлечение кадра превысило 24 секунды".into());
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return status
                .success()
                .then_some(())
                .ok_or_else(|| "FFmpeg не смог декодировать кадр".into());
        }
        std::thread::sleep(Duration::from_millis(60));
    }
}

fn cover_crop(image: DynamicImage, width: u32, height: u32) -> DynamicImage {
    let (source_width, source_height) = image.dimensions();
    let target_ratio = width as f64 / height as f64;
    let source_ratio = source_width as f64 / source_height.max(1) as f64;
    let (crop_width, crop_height) = if source_ratio > target_ratio {
        (
            (source_height as f64 * target_ratio).round() as u32,
            source_height,
        )
    } else {
        (
            source_width,
            (source_width as f64 / target_ratio).round() as u32,
        )
    };
    let x = source_width.saturating_sub(crop_width) / 2;
    let y = source_height.saturating_sub(crop_height) / 2;
    image
        .crop_imm(x, y, crop_width.max(1), crop_height.max(1))
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
}

fn ensure_running(cancelled: &AtomicBool) -> Result<(), String> {
    (!cancelled.load(Ordering::Acquire))
        .then_some(())
        .ok_or_else(|| "Генерация отменена".into())
}

fn safe_key(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() < 8
        || value.len() > 80
        || !value.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
    {
        return Err("Некорректный ключ обложки".into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn rejects_black_and_flat_frames() {
        let black = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(64, 64, Rgb([0, 0, 0])));
        let flat = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(64, 64, Rgb([128, 128, 128])));
        assert!(FrameAnalyzer::score(&black).is_none());
        assert!(FrameAnalyzer::score(&flat).is_none());
    }

    #[test]
    fn prefers_sharp_contrasting_frame() {
        let detailed = DynamicImage::ImageRgb8(ImageBuffer::from_fn(96, 54, |x, y| {
            if (x / 4 + y / 4) % 2 == 0 {
                Rgb([35, 60, 180])
            } else {
                Rgb([225, 190, 45])
            }
        }));
        let muted = DynamicImage::ImageRgb8(ImageBuffer::from_fn(96, 54, |x, _| {
            let value = 100 + (x % 8) as u8;
            Rgb([value, value, value])
        }));
        let detailed = FrameAnalyzer::score(&detailed).expect("detailed frame should be usable");
        assert!(FrameAnalyzer::score(&muted).is_none_or(|muted| detailed.total > muted.total));
    }

    #[test]
    fn creates_vertical_webp_cover() {
        let image = DynamicImage::ImageRgb8(ImageBuffer::from_fn(160, 90, |x, y| {
            Rgb([x as u8, y as u8, 120])
        }));
        let cover = cover_crop(image, 600, 900);
        assert_eq!(cover.dimensions(), (600, 900));
        let mut bytes = Vec::new();
        cover
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)
            .unwrap();
        assert!(bytes.starts_with(b"RIFF"));
    }

    #[test]
    fn cache_key_is_bounded_and_traversal_safe() {
        assert_eq!(safe_key("AABBCCDD-12").unwrap(), "aabbccdd-12");
        assert!(safe_key("../poster").is_err());
    }

    #[test]
    fn cache_generations_do_not_overwrite_each_other() {
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-thumbnail-cache-{}",
            std::process::id()
        ));
        let cache = ThumbnailCache::new(root.clone());
        let first = cache.store("aabbccdd", b"one").unwrap();
        let second = cache.store("aabbccdd", b"two").unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(cache.path(&first).unwrap()).unwrap(), b"one");
        assert_eq!(std::fs::read(cache.path(&second).unwrap()).unwrap(), b"two");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolver_only_uses_applied_generated_cover() {
        assert_eq!(
            ThumbnailResolver::selected(Some(("generated-a.webp", true)), Some("original.jpg")),
            Some("generated-a.webp")
        );
        assert_eq!(
            ThumbnailResolver::selected(Some(("generated-a.webp", false)), Some("original.jpg")),
            Some("original.jpg")
        );
    }

    #[test]
    fn cancellation_stops_before_spawning_ffmpeg() {
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-thumbnail-cancel-{}",
            std::process::id()
        ));
        let source = root.join("source.mp4");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&source, b"not-a-real-video").unwrap();
        let cancelled = Arc::new(AtomicBool::new(true));
        let error = ThumbnailEngine::new(
            root.join("missing-ffmpeg"),
            ThumbnailCache::new(root.join("cache")),
        )
        .generate(source.to_str().unwrap(), "aabbccdd", cancelled)
        .unwrap_err();
        assert_eq!(error, "Генерация отменена");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_non_local_video_source() {
        let error = ThumbnailEngine::new(
            PathBuf::from("missing-ffmpeg"),
            ThumbnailCache::new(std::env::temp_dir()),
        )
        .generate(
            "https://example.com/video.mp4",
            "aabbccdd",
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap_err();
        assert!(error.contains("локальным потоком"));
    }

    #[test]
    #[ignore = "requires PIRATE_CINEMA_FFMPEG and performs real video encoding"]
    fn generates_webp_from_real_mp4_and_mkv() {
        let ffmpeg = std::env::var_os("PIRATE_CINEMA_FFMPEG")
            .map(PathBuf::from)
            .expect("PIRATE_CINEMA_FFMPEG is required");
        assert!(ffmpeg.is_file());
        let root = std::env::temp_dir().join(format!(
            "pirate-cinema-thumbnail-live-{}-{}",
            std::process::id(),
            CACHE_GENERATION.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        for extension in ["mp4", "mkv"] {
            let source = root.join(format!("fixture.{extension}"));
            let status = Command::new(&ffmpeg)
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=640x360:rate=24",
                    "-t",
                    "12",
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-y",
                ])
                .arg(&source)
                .status()
                .unwrap();
            assert!(status.success(), "failed to create {extension} fixture");
            let sample = root.join(format!("sample-{extension}.png"));
            extract_frame(
                &ffmpeg,
                source.to_str().unwrap(),
                1,
                &sample,
                &AtomicBool::new(false),
            )
            .unwrap();
            let sample_image = image::open(&sample).unwrap();
            assert!(
                FrameAnalyzer::score(&sample_image).is_some(),
                "generated {extension} frame should pass quality filtering"
            );
            let generated =
                ThumbnailEngine::new(ffmpeg.clone(), ThumbnailCache::new(root.join("cache")))
                    .generate(
                        source.to_str().unwrap(),
                        if extension == "mp4" {
                            "aabbccdd"
                        } else {
                            "11223344"
                        },
                        Arc::new(AtomicBool::new(false)),
                    )
                    .unwrap();
            let image = image::load_from_memory(&generated.bytes).unwrap();
            assert_eq!(image.dimensions(), (600, 900));
            assert!(root.join("cache").join(generated.file_name).is_file());
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
