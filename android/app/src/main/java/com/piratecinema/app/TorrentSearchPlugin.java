package com.piratecinema.app;

import com.getcapacitor.JSArray;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.json.JSONArray;
import org.json.JSONObject;

@CapacitorPlugin(name = "TorrentSearch")
public class TorrentSearchPlugin extends Plugin {
    private static final String INDEX_URL = "https://releases.yourok.ru/torr/rutor.ls";
    private static final long MAX_INDEX_BYTES = 80L * 1024 * 1024;
    private static final long REFRESH_MS = 24L * 60 * 60 * 1000;
    private static final ExecutorService worker = Executors.newSingleThreadExecutor();
    private static final Pattern QUALITY = Pattern.compile("(?i)(2160p|4k|1080p|720p|480p)");

    static { System.loadLibrary("pirate_cinema_librqbit"); }
    private static native String nativeSearchIndex(String path, String query);

    @PluginMethod
    public void search(PluginCall call) {
        String query = call.getString("query", "").trim();
        if (query.codePointCount(0, query.length()) < 2) { call.reject("Введите минимум 2 символа"); return; }
        worker.execute(() -> {
            File candidate = null;
            try {
                try { candidate = ensureIndex(); }
                catch (Exception updateError) { candidate = indexFile(); if (candidate.length() <= 1024 * 1024) throw updateError; }
                JSArray results = searchIndex(candidate, query);
                if (!candidate.equals(indexFile())) { promoteIndex(candidate); candidate = indexFile(); }
                JSObject result = new JSObject();
                result.put("source", "RuTor"); result.put("updatedAt", candidate.lastModified()); result.put("results", results); call.resolve(result);
            } catch (Throwable error) { call.reject("Поиск RuTor недоступен", error.toString()); }
            finally { if (candidate != null && !candidate.equals(indexFile())) candidate.delete(); }
        });
    }

    private File ensureIndex() throws Exception {
        File directory = indexFile().getParentFile();
        if (!directory.exists() && !directory.mkdirs()) throw new IllegalStateException("Не удалось создать кэш поиска");
        File index = indexFile();
        File backup = backupFile();
        if (!index.exists() && backup.exists() && !backup.renameTo(index)) throw new IllegalStateException("Не удалось восстановить индекс RuTor");
        if (index.length() > 1024 * 1024 && System.currentTimeMillis() - index.lastModified() < REFRESH_MS) return index;
        File temporary = new File(directory, "rutor.tmp");
        HttpURLConnection connection = (HttpURLConnection) new URL(INDEX_URL).openConnection();
        connection.setConnectTimeout(15_000); connection.setReadTimeout(90_000); connection.setInstanceFollowRedirects(true); connection.setRequestProperty("User-Agent", "Pirate-Cinema-Android");
        try {
            int status = connection.getResponseCode(); long length = connection.getContentLengthLong();
            if (status != 200 || length > MAX_INDEX_BYTES) throw new IllegalStateException("Индекс RuTor: HTTP " + status);
            try (BufferedInputStream input = new BufferedInputStream(connection.getInputStream()); BufferedOutputStream output = new BufferedOutputStream(new FileOutputStream(temporary))) {
                byte[] buffer = new byte[64 * 1024]; long total = 0;
                for (int count; (count = input.read(buffer)) != -1;) { total += count; if (total > MAX_INDEX_BYTES) throw new IllegalStateException("Индекс RuTor слишком большой"); output.write(buffer, 0, count); }
                if (total < 1024 * 1024) throw new IllegalStateException("Индекс RuTor пуст");
            }
            return temporary;
        } catch (Exception error) { temporary.delete(); throw error; }
        finally { connection.disconnect(); }
    }

    private void promoteIndex(File candidate) throws Exception {
        File current = indexFile(), backup = backupFile();
        if (backup.exists() && !backup.delete()) throw new IllegalStateException("Не удалось очистить резервный индекс RuTor");
        if (current.exists() && !current.renameTo(backup)) throw new IllegalStateException("Не удалось сохранить резервный индекс RuTor");
        if (!candidate.renameTo(current)) {
            if (backup.exists()) backup.renameTo(current);
            throw new IllegalStateException("Не удалось заменить индекс RuTor");
        }
        if (backup.exists()) backup.delete();
    }

    private File indexFile() { return new File(new File(getContext().getCacheDir(), "rutor-search"), "rutor.ls"); }
    private File backupFile() { return new File(indexFile().getParentFile(), "rutor.bak"); }

    static JSArray searchIndex(File index, String query) throws Exception {
        JSONArray raw = new JSONArray(nativeSearchIndex(index.getAbsolutePath(), query)); JSArray output = new JSArray();
        for (int i = 0; i < raw.length(); i++) {
            JSONObject item = raw.getJSONObject(i); String title = item.optString("Title", item.optString("Name")); Matcher quality = QUALITY.matcher(title); int videoQuality = item.optInt("VideoQuality");
            String label = quality.find() ? quality.group(1).toLowerCase(Locale.ROOT).replace("4k", "2160p") : videoQuality >= 300 ? "2160p" : videoQuality >= 200 ? "1080p" : videoQuality >= 100 ? "720p" : "Другое";
            JSObject value = new JSObject();
            value.put("title", title); value.put("size", item.optString("Size", "—")); value.put("seeders", item.optInt("Seed")); value.put("peers", item.optInt("Peer"));
            value.put("magnet", item.optString("Magnet").replace("&amp;", "&")); value.put("hash", item.optString("Hash").toLowerCase(Locale.ROOT)); value.put("tracker", item.optString("Tracker", "RuTor"));
            value.put("source", "RuTor"); value.put("year", item.optInt("Year")); value.put("imdbId", item.optString("Imdbid")); value.put("quality", label); value.put("createdAt", item.optString("CreateDate")); output.put(value);
        }
        return output;
    }
}
