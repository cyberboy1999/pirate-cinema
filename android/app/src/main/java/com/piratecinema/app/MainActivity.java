package com.piratecinema.app;

import android.os.Bundle;
import android.content.Intent;
import androidx.core.splashscreen.SplashScreen;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        SplashScreen.installSplashScreen(this);
        registerPlugin(ExternalPlayerPlugin.class);
        registerPlugin(LibrqbitPlugin.class);
        registerPlugin(TorrentMediaPlugin.class);
        registerPlugin(MobileStoragePlugin.class);
        registerPlugin(PlatformToolsPlugin.class);
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
    }
}
