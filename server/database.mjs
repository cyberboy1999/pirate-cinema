import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { DatabaseSync } from "node:sqlite";

export class MediaDatabase {
  constructor(filePath) {
    mkdirSync(dirname(filePath), { recursive: true });
    this.db = new DatabaseSync(filePath);
    this.db.exec("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=3000;");
    this.migrate();
  }

  migrate() {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS media_items (
        torrent_hash TEXT PRIMARY KEY,
        torrent_name TEXT NOT NULL,
        imdb_id TEXT,
        title TEXT NOT NULL,
        original_title TEXT,
        year INTEGER,
        media_type TEXT NOT NULL DEFAULT 'unknown',
        poster_url TEXT,
        poster_path TEXT,
        runtime_seconds INTEGER,
        rating REAL,
        genres_json TEXT NOT NULL DEFAULT '[]',
        match_confidence REAL NOT NULL DEFAULT 0,
        playback_timecode INTEGER,
        playback_duration INTEGER,
        last_played_at TEXT,
        last_seen_on_server TEXT NOT NULL,
        added_at TEXT NOT NULL,
        torrent_status TEXT,
        download_speed INTEGER,
        upload_speed INTEGER,
        peers INTEGER,
        is_active INTEGER NOT NULL DEFAULT 0,
        is_started INTEGER NOT NULL DEFAULT 0,
        is_watched INTEGER NOT NULL DEFAULT 0
      );
      CREATE INDEX IF NOT EXISTS idx_media_items_last_seen ON media_items(last_seen_on_server DESC);
      CREATE INDEX IF NOT EXISTS idx_media_items_last_played ON media_items(last_played_at DESC) WHERE last_played_at IS NOT NULL;
      CREATE INDEX IF NOT EXISTS idx_media_items_imdb_id ON media_items(imdb_id) WHERE imdb_id IS NOT NULL;
      CREATE TABLE IF NOT EXISTS media_file_history (
        torrent_hash TEXT NOT NULL,
        file_index INTEGER NOT NULL,
        file_name TEXT NOT NULL,
        file_path TEXT,
        first_played_at TEXT NOT NULL,
        playback_timecode INTEGER,
        playback_duration INTEGER,
        is_watched INTEGER NOT NULL DEFAULT 0,
        last_played_at TEXT NOT NULL,
        launch_count INTEGER NOT NULL DEFAULT 1,
        PRIMARY KEY (torrent_hash,file_index)
      );
      CREATE INDEX IF NOT EXISTS idx_media_file_history_hash ON media_file_history(torrent_hash,last_played_at DESC);
    `);
    const columns=new Set(this.db.prepare("PRAGMA table_info(media_items)").all().map(row=>row.name));
    if(!columns.has("metadata_provider"))this.db.exec("ALTER TABLE media_items ADD COLUMN metadata_provider TEXT");
    if(!columns.has("provider_id"))this.db.exec("ALTER TABLE media_items ADD COLUMN provider_id TEXT");
    if(!columns.has("overview"))this.db.exec("ALTER TABLE media_items ADD COLUMN overview TEXT");
    if(!columns.has("overview_source_url")){
      this.db.exec("ALTER TABLE media_items ADD COLUMN overview_source_url TEXT");
      // One-time invalidation: retain descriptions/history, refresh using Russian-first matching.
      if(columns.has("metadata_checked_at"))this.db.exec("UPDATE media_items SET metadata_checked_at=NULL");
    }
    if(!columns.has("metadata_checked_at"))this.db.exec("ALTER TABLE media_items ADD COLUMN metadata_checked_at TEXT");
    if(!columns.has("audio_track_id"))this.db.exec("ALTER TABLE media_items ADD COLUMN audio_track_id INTEGER");
    this.db.exec("CREATE INDEX IF NOT EXISTS idx_media_items_provider ON media_items(metadata_provider,provider_id) WHERE provider_id IS NOT NULL;");
    const historyColumns=new Set(this.db.prepare("PRAGMA table_info(media_file_history)").all().map(row=>row.name));
    if(!historyColumns.has("playback_timecode"))this.db.exec("ALTER TABLE media_file_history ADD COLUMN playback_timecode INTEGER");
    if(!historyColumns.has("playback_duration"))this.db.exec("ALTER TABLE media_file_history ADD COLUMN playback_duration INTEGER");
    if(!historyColumns.has("is_watched"))this.db.exec("ALTER TABLE media_file_history ADD COLUMN is_watched INTEGER NOT NULL DEFAULT 0");
    this.db.exec("CREATE TABLE IF NOT EXISTS app_migrations (id TEXT PRIMARY KEY)");
    if (!this.db.prepare("SELECT id FROM app_migrations WHERE id=?").get("viewed-on-launch-v1")) {
      this.db.exec("BEGIN");
      try {
        this.db.exec("UPDATE media_file_history SET is_watched=1 WHERE launch_count>0");
        this.db.prepare("INSERT INTO app_migrations(id) VALUES (?)").run("viewed-on-launch-v1");
        this.db.exec("COMMIT");
      } catch (error) { this.db.exec("ROLLBACK"); throw error; }
    }
    this.db.exec("PRAGMA optimize;");
  }

  get(hash) { return this.db.prepare("SELECT * FROM media_items WHERE torrent_hash = ?").get(hash); }
  getMedia(hash) { const row=this.get(hash);return row?toMediaItem(row):null; }

  upsert(item) {
    const now = new Date().toISOString();
    this.db.prepare(`
      INSERT INTO media_items (
        torrent_hash,torrent_name,imdb_id,metadata_provider,provider_id,title,original_title,year,media_type,poster_url,poster_path,
        runtime_seconds,rating,genres_json,match_confidence,last_seen_on_server,added_at,torrent_status,
        download_speed,upload_speed,peers,is_active,is_started,is_watched
      ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
      ON CONFLICT(torrent_hash) DO UPDATE SET
        torrent_name=excluded.torrent_name,
        imdb_id=COALESCE(excluded.imdb_id,media_items.imdb_id),
        metadata_provider=COALESCE(excluded.metadata_provider,media_items.metadata_provider),provider_id=COALESCE(excluded.provider_id,media_items.provider_id),
        title=excluded.title,
        original_title=COALESCE(excluded.original_title,media_items.original_title),
        year=COALESCE(excluded.year,media_items.year), media_type=excluded.media_type,
        poster_url=COALESCE(excluded.poster_url,media_items.poster_url),
        poster_path=COALESCE(excluded.poster_path,media_items.poster_path),
        runtime_seconds=COALESCE(excluded.runtime_seconds,media_items.runtime_seconds),
        rating=COALESCE(excluded.rating,media_items.rating), genres_json=excluded.genres_json,
        match_confidence=MAX(excluded.match_confidence,media_items.match_confidence),
        last_seen_on_server=excluded.last_seen_on_server,torrent_status=excluded.torrent_status,
        download_speed=excluded.download_speed,upload_speed=excluded.upload_speed,peers=excluded.peers,
        is_active=excluded.is_active
    `).run(
      item.torrentHash,item.torrentName,null,item.metadataProvider??null,item.providerId??null,item.title,item.originalTitle ?? null,item.year ?? null,
      item.mediaType ?? "unknown",item.posterUrl ?? null,item.posterPath ?? null,item.runtimeSeconds ?? null,
      item.rating ?? null,JSON.stringify(item.genres ?? []),item.matchConfidence ?? 0,now,item.addedAt ?? now,
      item.torrentStatus ?? null,item.downloadSpeed ?? 0,item.uploadSpeed ?? 0,item.peers ?? 0,
      item.isActive ? 1 : 0,item.isStarted ? 1 : 0,item.isWatched ? 1 : 0
    );
  }

  updateViewed(hash, viewed) {
    this.db.prepare(`UPDATE media_items SET playback_timecode=?,playback_duration=?,last_played_at=?,is_started=?,is_watched=? WHERE torrent_hash=?`).run(
      viewed.timecode ?? null, viewed.duration ?? null, viewed.lastPlayedAt ?? new Date().toISOString(), viewed.timecode > 0 ? 1 : 0,
      viewed.duration > 0 && viewed.timecode / viewed.duration >= 0.92 ? 1 : 0, hash
    );
  }

  getAudioTrack(hash) { return this.db.prepare("SELECT audio_track_id AS audioTrackId FROM media_items WHERE torrent_hash=?").get(hash)?.audioTrackId ?? null; }
  saveAudioTrack(hash, audioTrackId) { if(Number.isSafeInteger(audioTrackId)&&audioTrackId>0)this.db.prepare("UPDATE media_items SET audio_track_id=? WHERE torrent_hash=?").run(audioTrackId,hash); }

  markFilePlayed(hash, file) {
    const now=new Date().toISOString();
    this.db.prepare(`
      INSERT INTO media_file_history (torrent_hash,file_index,file_name,file_path,first_played_at,last_played_at,launch_count,is_watched)
      VALUES (?,?,?,?,?,?,1,1)
      ON CONFLICT(torrent_hash,file_index) DO UPDATE SET
        file_name=excluded.file_name,file_path=COALESCE(excluded.file_path,media_file_history.file_path),
        last_played_at=excluded.last_played_at,launch_count=media_file_history.launch_count+1,is_watched=1
    `).run(hash,Number(file.fileIndex),String(file.fileName||"video"),file.filePath??null,now,now);
  }
  updateFileProgress(hash, fileIndex, progress) {
    const timecode=Math.max(0,Math.floor(Number(progress.timecode)||0));const duration=Math.max(0,Math.floor(Number(progress.duration)||0));
    const now=progress.lastPlayedAt??new Date().toISOString();
    this.db.prepare("UPDATE media_file_history SET playback_timecode=?,playback_duration=?,last_played_at=? WHERE torrent_hash=? AND file_index=?").run(timecode,duration,now,hash,Number(fileIndex));
    this.updateViewed(hash,{timecode,duration,lastPlayedAt:now});
  }


  setFileViewed(hash, fileIndex, viewed) {
    return this.db.prepare("UPDATE media_file_history SET is_watched=? WHERE torrent_hash=? AND file_index=?").run(viewed ? 1 : 0, hash, fileIndex).changes;
  }

  saveDescription(hash, overview, sourceUrl=null) {
    const current=this.get(hash);
    const russian=value=>(String(value??"").match(/[а-яё]/gi)?.length??0)>20;
    const replace=Boolean(overview?.trim())&&(!russian(current?.overview)||russian(overview));
    this.db.prepare("UPDATE media_items SET overview=?,overview_source_url=?,metadata_checked_at=? WHERE torrent_hash=?")
      .run(replace?overview.trim():current?.overview??null,replace?sourceUrl:current?.overview_source_url??null,new Date().toISOString(),hash);
  }

  resetFileProgress(hash, fileIndex) {
    const latest = this.listFileHistory(hash)[0];
    this.db.prepare("UPDATE media_file_history SET playback_timecode=0 WHERE torrent_hash=? AND file_index=?").run(hash, fileIndex);
    if (latest?.fileIndex === fileIndex) this.updateViewed(hash, { timecode: 0, duration: latest.playbackDuration, lastPlayedAt: latest.lastPlayedAt });
  }

  listFileHistory(hash) {
    return this.db.prepare("SELECT file_index AS fileIndex,file_name AS fileName,file_path AS filePath,playback_timecode AS playbackTimecode,playback_duration AS playbackDuration,is_watched AS isWatched,last_played_at AS lastPlayedAt,launch_count AS launchCount FROM media_file_history WHERE torrent_hash=? ORDER BY last_played_at DESC").all(hash);
  }

  remove(hash) { this.db.prepare("DELETE FROM media_file_history WHERE torrent_hash = ?").run(hash);return this.db.prepare("DELETE FROM media_items WHERE torrent_hash = ?").run(hash).changes; }

  reconcile(serverHashes) {
    const hashes=[...new Set(serverHashes.filter(Boolean))];
    if(!hashes.length){this.db.prepare("DELETE FROM media_file_history").run();return this.db.prepare("DELETE FROM media_items").run().changes}
    const placeholders=hashes.map(()=>"?").join(",");
    this.db.prepare(`DELETE FROM media_file_history WHERE torrent_hash NOT IN (${placeholders})`).run(...hashes);
    return this.db.prepare(`DELETE FROM media_items WHERE torrent_hash NOT IN (${placeholders})`).run(...hashes).changes;
  }

  list() { return this.db.prepare("SELECT * FROM media_items ORDER BY added_at DESC").all().map(toMediaItem); }
  stats() { return this.db.prepare("SELECT COUNT(*) AS total, SUM(is_active) AS active, SUM(CASE WHEN playback_timecode > 0 AND (playback_duration IS NULL OR playback_duration=0 OR CAST(playback_timecode AS REAL)/playback_duration < 0.92) THEN 1 ELSE 0 END) AS continuing FROM media_items").get(); }
  checkpoint() { this.db.exec("PRAGMA wal_checkpoint(FULL)"); }
  close() { this.db.close(); }
}

function toMediaItem(row) {
  const progress = row.playback_duration > 0 ? Math.max(0, Math.min(100, Math.round(row.playback_timecode / row.playback_duration * 100))) : null;
  return {
    torrentHash:row.torrent_hash,torrentName:row.torrent_name,metadataProvider:row.metadata_provider,providerId:row.provider_id,title:row.title,
    originalTitle:row.original_title,year:row.year,mediaType:row.media_type,posterUrl:row.poster_url,
    posterPath:row.poster_path ? `/api/posters/${encodeURIComponent(row.poster_path)}` : null,
    runtimeSeconds:row.runtime_seconds,rating:row.rating,genres:JSON.parse(row.genres_json || "[]"),overview:row.overview??"",overviewSourceUrl:row.overview_source_url??null,
    matchConfidence:row.match_confidence,progress,lastPlayedAt:row.last_played_at,addedAt:row.added_at,
    torrentStatus:row.torrent_status,downloadSpeed:row.download_speed,uploadSpeed:row.upload_speed,peers:row.peers,
    isActive:Boolean(row.is_active),isStarted:Boolean(row.is_started),isWatched:Boolean(row.is_watched)
  };
}
