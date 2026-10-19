package com.piratecinema.app;

import android.app.Activity;
import android.content.Intent;
import android.os.SystemClock;
import android.view.*;
import android.webkit.WebView;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.json.*;
import org.junit.Test;
import org.junit.runner.RunWith;
import java.util.concurrent.*;
import static org.junit.Assert.*;

/** Opt-in actual 20-minute QA playback; requires the owned 1205-second fixture already added. */
@RunWith(AndroidJUnit4.class)
public class TorrentSoakTest {
    private WebView web;
    private static WebView find(View view){if(view instanceof WebView)return (WebView)view;if(view instanceof ViewGroup){ViewGroup group=(ViewGroup)view;for(int i=0;i<group.getChildCount();i++){WebView result=find(group.getChildAt(i));if(result!=null)return result;}}return null;}
    private String js(String expression)throws Exception{
        CountDownLatch done=new CountDownLatch(1);String[] result={null};
        InstrumentationRegistry.getInstrumentation().runOnMainSync(()->{
            for(Activity current:androidx.test.runner.lifecycle.ActivityLifecycleMonitorRegistry.getInstance().getActivitiesInStage(androidx.test.runner.lifecycle.Stage.RESUMED)){
                WebView visible=find(current.getWindow().getDecorView());if(visible!=null)web=visible;
            }
            web.evaluateJavascript(expression,value->{result[0]=value;done.countDown();});
        });
        assertTrue("WebView callback timeout",done.await(10,TimeUnit.SECONDS));return result[0];
    }
    private void until(String expression,long milliseconds)throws Exception{long end=SystemClock.elapsedRealtime()+milliseconds;while(SystemClock.elapsedRealtime()<end){if("true".equals(js(expression)))return;Thread.sleep(250);}fail("Condition timed out: "+expression);}
    @Test public void realPlaybackTwentyMinutes()throws Exception{
        org.junit.Assume.assumeTrue("Explicit opt-in only", "true".equals(InstrumentationRegistry.getArguments().getString("soak")));
        var instrumentation=InstrumentationRegistry.getInstrumentation();
        var context=instrumentation.getTargetContext();
        assertTrue("Never run soak against user profile",context.getPackageName().endsWith(".qa"));
        Activity activity=instrumentation.startActivitySync(new Intent(context,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        instrumentation.runOnMainSync(()->web=find(activity.getWindow().getDecorView()));assertNotNull(web);
        until("document.body.innerText.includes('librqbit')",60000);
        js("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Начать')?.click()");
        until("Boolean(document.querySelector('button[aria-label=\"Открыть librqbit-owned-fixture.mp4\"]'))",60000);
        js("document.querySelector('button[aria-label=\"Открыть librqbit-owned-fixture.mp4\"]').click()");
        until("[...document.querySelectorAll('button')].some(b=>['Воспроизвести','Сначала'].includes(b.textContent.trim()))",10000);
        js("[...document.querySelectorAll('button')].find(b=>['Воспроизвести','Сначала'].includes(b.textContent.trim())).click()");
        until("document.querySelector('video')?.readyState===4",90000);
        long start=SystemClock.elapsedRealtime(),end=start+20*60*1000;double previous=-1;long peakPss=0;
        while(SystemClock.elapsedRealtime()<end){
            String raw=js("JSON.stringify((()=>{const v=document.querySelector('video');return {time:v.currentTime,ready:v.readyState,paused:v.paused,width:v.videoWidth,frames:v.getVideoPlaybackQuality().totalVideoFrames,error:v.error?.message}})())");
            JSONObject state=new JSONObject(new JSONArray("["+raw+"]").getString(0));
            assertFalse(state.toString(),state.getBoolean("paused"));assertTrue(state.toString(),state.getInt("width")>0);assertTrue(state.toString(),state.getDouble("time")>previous);previous=state.getDouble("time");
            peakPss=Math.max(peakPss,android.os.Debug.getPss());assertTrue("PSS exceeded 1 GiB",peakPss<1024*1024);
            System.out.println("SOAK elapsed="+((SystemClock.elapsedRealtime()-start)/1000)+" state="+state+" peakPssKiB="+peakPss);
            Thread.sleep(Math.min(30000,Math.max(1,end-SystemClock.elapsedRealtime())));
        }
        assertTrue("Not enough real playback",previous>=1160);
        System.out.println("PASS actual 20-minute playback; peakPssKiB="+peakPss);
        js("document.querySelector('button[aria-label=\"Закрыть Artplayer\"]')?.click()");
    }
}
