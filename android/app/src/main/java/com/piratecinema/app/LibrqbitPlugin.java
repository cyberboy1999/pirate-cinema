package com.piratecinema.app;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import android.content.Intent;
import androidx.core.content.ContextCompat;
import java.util.concurrent.Executors;
import java.util.concurrent.ExecutorService;

@CapacitorPlugin(name = "Librqbit")
public class LibrqbitPlugin extends Plugin {
    private static final String URL = "http://127.0.0.1:3030";
    private static final ExecutorService lifecycle=Executors.newSingleThreadExecutor();

    static {
        System.loadLibrary("pirate_cinema_librqbit");
    }

    private static native boolean nativeStart(String dataDirectory);
    private static native boolean nativeStop();

    @PluginMethod
    public void initialize(PluginCall call) {
        ContextCompat.startForegroundService(getContext(),new Intent(getContext(),TorrentService.class));
        lifecycle.execute(() -> {
            try {
                nativeStart(getContext().getFilesDir().getAbsolutePath() + "/librqbit");
                JSObject result = new JSObject();
                result.put("url", URL);
                result.put("version", "librqbit 9.0.1");
                call.resolve(result);
            } catch (Throwable error) {
                getContext().stopService(new Intent(getContext(),TorrentService.class));
                android.util.Log.e("Librqbit", "Engine initialization failed", error);
                call.reject("Не удалось запустить librqbit", error.toString());
            }
        });
    }

    @PluginMethod
    public void shutdown(PluginCall call) {
        getContext().stopService(new Intent(getContext(),TorrentService.class));
        lifecycle.execute(() -> {
            try {
                nativeStop();
                call.resolve();
            } catch (Throwable error) {
                call.reject("Не удалось остановить librqbit", error.toString());
            }
        });
    }

    static void stopOwnedEngine(){lifecycle.execute(()->{try{nativeStop();}catch(Throwable error){android.util.Log.e("Librqbit","Shutdown failed",error);}});}

    @PluginMethod public void storage(PluginCall call){
        Integer quota=call.getInt("quotaGiB");
        if(quota!=null){if(quota<1||quota>128){call.reject("Лимит должен быть от 1 до 128 ГБ");return;}getContext().getSharedPreferences("engine",0).edit().putInt("quotaGiB",quota).apply();}
        JSObject result=new JSObject();result.put("quotaGiB",getContext().getSharedPreferences("engine",0).getInt("quotaGiB",8));result.put("usedBytes",TorrentService.allocatedBytes);result.put("availableBytes",TorrentService.availableBytes);result.put("warning",TorrentService.storageWarning);call.resolve(result);
    }
}
