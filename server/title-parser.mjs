const TECHNICAL_PATTERNS = [
  /\b(?:2160p|1080p|720p|480p|4k|uhd)\b/gi,
  /\b(?:web[ ._-]?dl|webrip|blu[ ._-]?ray|bdrip|brrip|hdrip|remux|dvdrip)\b/gi,
  /\b(?:x26[45]|h[ ._-]?26[45]|hevc|av1)\b/gi,
  /\b(?:hdr10\+?|hdr|dolby[ ._-]?vision|dovi|dv)\b/gi,
  /\b(?:atmos|aac|dts(?:[ ._-]?hd)?|truehd|ddp(?:5[ ._-]?1)?|ac3)\b/gi,
  /\b(?:multi|rus|eng|ita|ger|repack|proper|extended|unrated)\b/gi,
  /\b(?:lostfilm|hdrezka|newstudio|alexfilm)\b/gi,
  /\b(?:s\d{1,2}(?:e\d{1,3})?|season[ ._-]?\d{1,2}|episode[ ._-]?\d{1,3})\b/gi,
  /\b(?:mkv|mp4|avi|mov)\b/gi,
];

export function normalizeTitle(value = "") {
  return value
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/&/g, " and ")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim()
    .replace(/\s+/g, " ");
}

export function parseTorrentTitle(rawTitle = "") {
  const fileName = rawTitle.split(/[\\/]/).pop() ?? rawTitle;
  const yearMatches = [...fileName.matchAll(/(?:^|[^0-9])((?:19|20)\d{2})(?!\d)/g)];
  const year = yearMatches.length ? Number(yearMatches[0][1]) : null;
  let cleaned = fileName.replace(/\.[a-z0-9]{2,4}$/i, " ");
  if (yearMatches.length) cleaned = cleaned.slice(0, yearMatches[0].index).trim();
  for (const pattern of TECHNICAL_PATTERNS) cleaned = cleaned.replace(pattern, " ");
  cleaned = cleaned
    .replace(/[._()[\]{}-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return { title: cleaned || rawTitle, year, normalizedTitle: normalizeTitle(cleaned || rawTitle) };
}

export function localizedTorrentTitle(rawTitle = "") {
  const firstTitle=String(rawTitle).split(/\s+\/\s+/)[0].trim();
  if(!/[А-Яа-яЁё]/.test(firstTitle))return null;
  return firstTitle.replace(/^\d{1,3}\s+(?=[А-Яа-яЁё])/u,"").replace(/\s*\((?:19|20)\d{2}\).*$/,"").trim()||null;
}

function tokenSimilarity(left, right) {
  const a = new Set(normalizeTitle(left).split(" ").filter(Boolean));
  const b = new Set(normalizeTitle(right).split(" ").filter(Boolean));
  if (!a.size || !b.size) return 0;
  const common = [...a].filter((token) => b.has(token)).length;
  return (2 * common) / (a.size + b.size);
}

export function matchConfidence(parsed, candidate) {
  const titleScore = tokenSimilarity(parsed.title, candidate.title);
  const exactBoost = normalizeTitle(parsed.title) === normalizeTitle(candidate.title) ? 0.12 : 0;
  const yearScore = !parsed.year || !candidate.year ? 0.05 : Math.abs(parsed.year - candidate.year) === 0 ? 0.16 : Math.abs(parsed.year - candidate.year) === 1 ? 0.06 : -0.14;
  const typeScore = candidate.type === "movie" || candidate.type === "tvSeries" ? 0.03 : 0;
  return Math.max(0, Math.min(1, titleScore * 0.72 + exactBoost + yearScore + typeScore));
}
