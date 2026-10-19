package com.piratecinema.app;

import android.app.Activity;
import android.content.Intent;
import android.webkit.WebView;
import android.view.*;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.junit.Test;
import org.junit.runner.RunWith;
import java.net.*;
import java.util.concurrent.*;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public class EngineLifecycleTest {
    private static boolean engine(){try{HttpURLConnection connection=(HttpURLConnection)new URL("http://127.0.0.1:3030/").openConnection();connection.setConnectTimeout(500);connection.setReadTimeout(500);try{return connection.getResponseCode()==200;}finally{connection.disconnect();}}catch(Exception error){return false;}}
    private static WebView find(View view){if(view instanceof WebView)return (WebView)view;if(view instanceof ViewGroup){ViewGroup group=(ViewGroup)view;for(int i=0;i<group.getChildCount();i++){WebView found=find(group.getChildAt(i));if(found!=null)return found;}}return null;}
    private static void waitEngine(boolean expected)throws Exception{long end=System.currentTimeMillis()+15000;while(System.currentTimeMillis()<end){if(engine()==expected)return;Thread.sleep(200);}assertEquals("Engine lifecycle timeout",expected,engine());}
    @Test public void stopRestartAndBackgroundPreserveOwnedEngine()throws Exception{
        var instrumentation=InstrumentationRegistry.getInstrumentation();var context=instrumentation.getTargetContext();
        org.junit.Assume.assumeTrue("Isolated QA only",context.getPackageName().endsWith(".qa"));
        Activity activity=instrumentation.startActivitySync(new Intent(context,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        waitEngine(true);
        context.startService(new Intent(context,TorrentService.class).setAction("stop"));waitEngine(false);
        CountDownLatch clicked=new CountDownLatch(1);
        instrumentation.runOnMainSync(()->{WebView web=find(activity.getWindow().getDecorView());assertNotNull(web);web.evaluateJavascript("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Настройки').click();setTimeout(()=>[...document.querySelectorAll('button')].find(b=>b.textContent.includes('Перезапустить встроенный движок')).click(),100)",value->clicked.countDown());});
        assertTrue(clicked.await(5,TimeUnit.SECONDS));waitEngine(true);
        instrumentation.runOnMainSync(()->activity.moveTaskToBack(true));Thread.sleep(3000);assertTrue("Foreground engine must survive app background",engine());
        System.out.println("PASS notification stop, in-app restart and background engine");
    }
}
