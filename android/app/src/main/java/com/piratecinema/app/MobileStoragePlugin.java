package com.piratecinema.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import androidx.activity.result.ActivityResult;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.ActivityCallback;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

@CapacitorPlugin(name = "MobileStorage")
public class MobileStoragePlugin extends Plugin {
    @PluginMethod
    public void exportBackup(PluginCall call) {
        String content = call.getString("content", "");
        if (content.length() > 5_000_000) { call.reject("Резервная копия слишком большая"); return; }
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("application/json")
            .putExtra(Intent.EXTRA_TITLE, call.getString("fileName", "pirate-cinema-backup.json"));
        startActivityForResult(call, intent, "exportResult");
    }

    @ActivityCallback
    private void exportResult(PluginCall call, ActivityResult result) {
        if (call == null) return;
        Uri uri = result.getData() == null ? null : result.getData().getData();
        if (result.getResultCode() != Activity.RESULT_OK || uri == null) { call.resolve(new JSObject().put("canceled", true)); return; }
        try (OutputStream stream = getContext().getContentResolver().openOutputStream(uri, "wt")) {
            if (stream == null) throw new IllegalStateException("Файл недоступен для записи");
            stream.write(call.getString("content", "").getBytes(StandardCharsets.UTF_8));
            call.resolve(new JSObject().put("ok", true));
        } catch (Exception error) { call.reject("Не удалось сохранить резервную копию", error); }
    }

    @PluginMethod
    public void importBackup(PluginCall call) {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("application/json");
        startActivityForResult(call, intent, "importResult");
    }

    @ActivityCallback
    private void importResult(PluginCall call, ActivityResult result) {
        if (call == null) return;
        Uri uri = result.getData() == null ? null : result.getData().getData();
        if (result.getResultCode() != Activity.RESULT_OK || uri == null) { call.resolve(new JSObject().put("canceled", true)); return; }
        try (InputStream stream = getContext().getContentResolver().openInputStream(uri); ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            if (stream == null) throw new IllegalStateException("Файл недоступен для чтения");
            byte[] buffer = new byte[8192]; int count, total = 0;
            while ((count = stream.read(buffer)) != -1) { total += count; if (total > 5_000_000) throw new IllegalStateException("Файл слишком большой"); output.write(buffer, 0, count); }
            call.resolve(new JSObject().put("content", new String(output.toByteArray(), StandardCharsets.UTF_8)));
        } catch (Exception error) { call.reject("Не удалось прочитать резервную копию", error); }
    }
}
