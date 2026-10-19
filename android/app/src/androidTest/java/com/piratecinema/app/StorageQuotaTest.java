package com.piratecinema.app;

import android.content.Intent;
import android.system.Os;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.json.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public class StorageQuotaTest {
    private static JSONObject request(String method,String path,byte[] body)throws Exception{
        HttpURLConnection connection=(HttpURLConnection)new URL("http://127.0.0.1:3030"+path).openConnection();connection.setRequestMethod(method);connection.setConnectTimeout(3000);connection.setReadTimeout(5000);
        try{if(body!=null){connection.setDoOutput(true);try(OutputStream output=connection.getOutputStream()){output.write(body);}}assertEquals(path,200,connection.getResponseCode());try(InputStream input=connection.getInputStream();ByteArrayOutputStream output=new ByteArrayOutputStream()){byte[] buffer=new byte[4096];int count;while((count=input.read(buffer))!=-1)output.write(buffer,0,count);return new JSONObject(new String(output.toByteArray(),StandardCharsets.UTF_8));}}finally{connection.disconnect();}
    }
    @Test public void alreadyPausedJobCannotPreventOtherJobsBeingPaused()throws Exception{
        var instrumentation=InstrumentationRegistry.getInstrumentation();var context=instrumentation.getTargetContext();
        org.junit.Assume.assumeTrue("Isolated QA only",context.getPackageName().endsWith(".qa"));
        instrumentation.startActivitySync(new Intent(context,MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        long ready=System.currentTimeMillis()+15000;while(System.currentTimeMillis()<ready){try{request("GET","/",null);break;}catch(Exception error){Thread.sleep(200);}}
        JSONArray listed=request("GET","/torrents",null).getJSONArray("torrents");assertTrue("Add the owned long fixture before this test",listed.length()>0);int first=listed.getJSONObject(0).getInt("id");
        try{request("POST","/torrents/"+first+"/pause",null);}catch(AssertionError alreadyPaused){assertEquals("paused",request("GET","/torrents/"+first+"/stats/v1",null).getString("state"));}
        String name="owned-quota-second.bin";ByteArrayOutputStream torrent=new ByteArrayOutputStream();torrent.write(("d4:infod6:lengthi1e4:name"+name.length()+":"+name+"12:piece lengthi16384e6:pieces20:").getBytes(StandardCharsets.US_ASCII));torrent.write(MessageDigest.getInstance("SHA-1").digest(new byte[]{0}));torrent.write("ee".getBytes(StandardCharsets.US_ASCII));
        int second=request("POST","/torrents",torrent.toByteArray()).getInt("id");
        JSONArray ordered=request("GET","/torrents",null).getJSONArray("torrents");assertEquals(2,ordered.length());
        int guardFirst=ordered.getJSONObject(0).getInt("id"),guardLater=ordered.getJSONObject(1).getInt("id");
        if(!"paused".equals(request("GET","/torrents/"+guardFirst+"/stats/v1",null).getString("state")))request("POST","/torrents/"+guardFirst+"/pause",null);
        if(!"live".equals(request("GET","/torrents/"+guardLater+"/stats/v1",null).getString("state")))request("POST","/torrents/"+guardLater+"/start",null);
        File directory=new File(context.getFilesDir(),"librqbit/downloads");assertTrue(directory.isDirectory());File allocation=File.createTempFile("owned-quota-allocation-",".bin",directory);
        var preferences=context.getSharedPreferences("engine",0);int previous=preferences.getInt("quotaGiB",8);
        try{
            assertTrue("Need 3 GiB free to safely run quota fixture",directory.getUsableSpace()>3L*1024*1024*1024);
            try(RandomAccessFile file=new RandomAccessFile(allocation,"rw")){Os.posix_fallocate(file.getFD(),0,1024L*1024*1024+4096);}
            preferences.edit().putInt("quotaGiB",1).commit();
            long end=System.currentTimeMillis()+20000;boolean paused=false;while(System.currentTimeMillis()<end){if("paused".equals(request("GET","/torrents/"+guardLater+"/stats/v1",null).getString("state"))){paused=true;break;}Thread.sleep(500);}
            assertTrue("Guard must continue past the already-paused first torrent",paused);assertFalse(TorrentService.storageWarning.isEmpty());
            System.out.println("PASS quota pauses later jobs despite already-paused first job");
        }finally{
            assertTrue("Remove only the owned allocation fixture",allocation.delete());preferences.edit().putInt("quotaGiB",previous).commit();request("POST","/torrents/"+second+"/delete",null);
        }
    }
}
