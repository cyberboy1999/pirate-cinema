package com.piratecinema.app;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.net.Uri;
import androidx.activity.result.ActivityResult;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.ActivityCallback;
import com.getcapacitor.annotation.CapacitorPlugin;

@CapacitorPlugin(name = "ExternalPlayer")
public class ExternalPlayerPlugin extends Plugin {
    @PluginMethod
    public void open(PluginCall call) {
        String url = call.getString("url", "");
        Uri uri = Uri.parse(url);
        if (!("http".equals(uri.getScheme()) || "https".equals(uri.getScheme()))) {
            call.reject("Разрешены только HTTP(S)-потоки");
            return;
        }

        Intent intent = new Intent(Intent.ACTION_VIEW).setDataAndType(uri, "video/*");
        intent.addCategory(Intent.CATEGORY_BROWSABLE);
        intent.putExtra("title", call.getString("title", "Pirate Cinema"));
        Integer position = call.getInt("positionMs", 0);
        if (position != null && position > 0) intent.putExtra("position", position);
        boolean preferMpv = "mpv".equals(call.getString("preferred", "mpv"));
        if (preferMpv) intent.setClassName("is.xyz.mpv", "is.xyz.mpv.MPVActivity");

        try {
            startActivityForResult(call, preferMpv ? intent : Intent.createChooser(intent, "Открыть с помощью"), "playerResult");
        } catch (ActivityNotFoundException | SecurityException error) {
            if (!preferMpv) {
                call.reject("На устройстве нет подходящего видеоплеера");
                return;
            }
            intent.setComponent(null);
            try {
                startActivityForResult(call, Intent.createChooser(intent, "Открыть с помощью"), "playerResult");
            } catch (ActivityNotFoundException | SecurityException fallbackError) {
                call.reject("Установите mpv-android или другой видеоплеер");
                return;
            }
        }
    }

    @ActivityCallback
    private void playerResult(PluginCall call, ActivityResult activityResult) {
        if (call == null) return;
        Intent data = activityResult.getData();
        JSObject result = new JSObject();
        result.put("ok", activityResult.getResultCode() == Activity.RESULT_OK);
        if (data != null && data.hasExtra("position")) result.put("positionMs", Math.max(0, data.getIntExtra("position", 0)));
        if (data != null && data.hasExtra("duration")) result.put("durationMs", Math.max(0, data.getIntExtra("duration", 0)));
        result.put("completed", activityResult.getResultCode() == Activity.RESULT_OK && (data == null || !data.hasExtra("position")));
        call.resolve(result);
    }
}
