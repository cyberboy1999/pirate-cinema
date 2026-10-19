package com.piratecinema.app;

import android.app.UiModeManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.Uri;
import java.net.Inet4Address;
import java.net.NetworkInterface;
import java.util.Collections;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

@CapacitorPlugin(name = "PlatformTools")
public class PlatformToolsPlugin extends Plugin {
    private boolean installed(String packageName) {
        try { getContext().getPackageManager().getPackageInfo(packageName, 0); return true; }
        catch (PackageManager.NameNotFoundException error) { return false; }
    }

    @PluginMethod
    public void openDependency(PluginCall call) {
        String dependency = call.getString("dependency", "");
        String packageName;
        String download;
        if ("mpv".equals(dependency)) {
            packageName = "is.xyz.mpv";
            download = "https://github.com/mpv-android/mpv-android/releases/latest";
        } else { call.reject("Неизвестное приложение"); return; }
        Intent intent = getContext().getPackageManager().getLaunchIntentForPackage(packageName);
        if (intent == null) intent = new Intent(Intent.ACTION_VIEW, Uri.parse(download));
        try { getActivity().startActivity(intent); call.resolve(); }
        catch (android.content.ActivityNotFoundException | SecurityException error) {
            call.reject("Не удалось открыть приложение или страницу установки");
        }
    }
    @PluginMethod
    public void launchMagnet(PluginCall call) {
        Intent intent = getActivity().getIntent();
        Uri data = intent == null ? null : intent.getData();
        JSObject result = new JSObject();
        if (data != null && "magnet".equals(data.getScheme())) {
            result.put("magnet", data.toString());
            intent.setData(null);
        }
        call.resolve(result);
    }

    @Override
    protected void handleOnNewIntent(Intent intent) {
        Uri data = intent == null ? null : intent.getData();
        if (data != null && "magnet".equals(data.getScheme())) {
            JSObject event = new JSObject();
            event.put("magnet", data.toString());
            intent.setData(null);
            notifyListeners("magnetReceived", event);
        }
    }

    @PluginMethod
    public void share(PluginCall call) {
        String url = call.getString("url", "");
        Uri uri = Uri.parse(url);
        if (!("http".equals(uri.getScheme()) || "https".equals(uri.getScheme()))) {
            call.reject("Разрешены только HTTP(S)-потоки");
            return;
        }
        Intent intent = new Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, url);
        getActivity().startActivity(Intent.createChooser(intent, call.getString("title", "Pirate Cinema")));
        call.resolve();
    }

    @PluginMethod
    public void openUrl(PluginCall call) {
        Uri uri = Uri.parse(call.getString("url", ""));
        if (!("http".equals(uri.getScheme()) || "https".equals(uri.getScheme()))) {
            call.reject("Разрешены только HTTP(S)-ссылки");
            return;
        }
        try { getActivity().startActivity(new Intent(Intent.ACTION_VIEW, uri)); call.resolve(); }
        catch (Exception error) { call.reject("Не удалось открыть ссылку"); }
    }

    @PluginMethod
    public void device(PluginCall call) {
        Context context = getContext();
        ConnectivityManager manager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        NetworkCapabilities capabilities = manager.getNetworkCapabilities(manager.getActiveNetwork());
        boolean network = capabilities != null;
        UiModeManager modes = (UiModeManager) context.getSystemService(Context.UI_MODE_SERVICE);
        JSObject result = new JSObject();
        result.put("network", network);
        result.put("mpv", installed("is.xyz.mpv"));
        result.put("tv", modes.getCurrentModeType() == Configuration.UI_MODE_TYPE_TELEVISION);
        try {
            for (NetworkInterface networkInterface : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                for (java.net.InetAddress address : Collections.list(networkInterface.getInetAddresses())) {
                    if (address instanceof Inet4Address && !address.isLoopbackAddress() && address.isSiteLocalAddress()) {
                        String host = address.getHostAddress();
                        result.put("subnet", host.substring(0, host.lastIndexOf('.') + 1));
                        break;
                    }
                }
            }
        } catch (Exception ignored) { }
        call.resolve(result);
    }
}
