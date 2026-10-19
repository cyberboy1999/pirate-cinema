package com.piratecinema.app;

import android.app.*;
import android.content.Intent;
import android.os.*;
import android.system.Os;
import androidx.core.app.NotificationCompat;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.*;
import org.json.*;

/** Visible ownership of the in-process engine; never restarts downloads after a force-stop. */
public class TorrentService extends Service {
    private final ScheduledExecutorService worker=Executors.newSingleThreadScheduledExecutor();
    private static final String CHANNEL="torrent-engine";
    static volatile long allocatedBytes,availableBytes;
    static volatile String storageWarning="";
    private volatile boolean stopping;

    @Override public void onCreate(){
        super.onCreate();
        if(Build.VERSION.SDK_INT>=26)getSystemService(NotificationManager.class).createNotificationChannel(new NotificationChannel(CHANNEL,"Локальный торрент-движок",NotificationManager.IMPORTANCE_LOW));
        Intent open=new Intent(this,MainActivity.class);
        Intent stop=new Intent(this,TorrentService.class).setAction("stop");
        Notification notification=new NotificationCompat.Builder(this,CHANNEL).setSmallIcon(android.R.drawable.stat_sys_download).setContentTitle("Pirate Cinema").setContentText("librqbit работает на устройстве").setOngoing(true)
            .setContentIntent(PendingIntent.getActivity(this,0,open,PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT))
            .addAction(0,"Остановить движок",PendingIntent.getService(this,1,stop,PendingIntent.FLAG_IMMUTABLE|PendingIntent.FLAG_UPDATE_CURRENT)).build();
        if(Build.VERSION.SDK_INT>=29)startForeground(61,notification,android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);else startForeground(61,notification);
        worker.scheduleWithFixedDelay(this::checkStorage,5,5,TimeUnit.SECONDS);
    }
    @Override public int onStartCommand(Intent intent,int flags,int startId){if(intent!=null&&"stop".equals(intent.getAction()))stopSelf();return START_NOT_STICKY;}
    @Override public IBinder onBind(Intent intent){return null;}
    @Override public void onTaskRemoved(Intent rootIntent){stopSelf();}
    @Override public void onTimeout(int startId,int type){stopSelf();}
    @Override public void onDestroy(){stopping=true;worker.shutdownNow();LibrqbitPlugin.stopOwnedEngine();super.onDestroy();}

    private void checkStorage(){try{
        File root=new File(getFilesDir(),"librqbit/downloads");
        allocatedBytes=allocated(root,root.getCanonicalPath()+File.separator);
        availableBytes=new StatFs(getFilesDir().getAbsolutePath()).getAvailableBytes();
        long quota=getSharedPreferences("engine",MODE_PRIVATE).getInt("quotaGiB",8)*1024L*1024*1024;
        String warning=storageWarning(allocatedBytes,availableBytes,quota);
        storageWarning=warning;
        if(!warning.isEmpty()&&!stopping){JSONObject list=new JSONObject(request("GET","/torrents"));JSONArray torrents=list.getJSONArray("torrents");for(int i=0;i<torrents.length()&&!stopping;i++){try{request("POST","/torrents/"+torrents.getJSONObject(i).getInt("id")+"/pause");}catch(Exception error){android.util.Log.w("TorrentService","Pause job: "+error.getMessage());}}}
    }catch(Exception error){if(!stopping)android.util.Log.w("TorrentService","Storage guard: "+error.getMessage());}}
    static String storageWarning(long used,long available,long quota){return available<512L*1024*1024?"Мало свободного места (нужны 512 МБ). Загрузки приостановлены.":used>=quota?"Достигнут лимит торрент-данных. Удалите файлы или увеличьте лимит; затем продолжите загрузку.":"";}
    static long allocated(File file,String root)throws Exception{
        if(!file.exists())return 0;
        String canonical=file.getCanonicalPath();if(!canonical.startsWith(root)&&!canonical.equals(root.substring(0,root.length()-1)))return 0;
        if(file.isFile())return Os.stat(canonical).st_blocks*512L;
        long bytes=0;File[] files=file.listFiles();if(files!=null)for(File child:files)bytes+=allocated(child,root);return bytes;
    }
    private static String request(String method,String path)throws Exception{
        HttpURLConnection connection=(HttpURLConnection)new URL("http://127.0.0.1:3030"+path).openConnection();
        connection.setConnectTimeout(1500);connection.setReadTimeout(2000);connection.setRequestMethod(method);
        try{if(connection.getResponseCode()!=200)throw new IOException("Engine HTTP "+connection.getResponseCode());try(InputStream input=connection.getInputStream();ByteArrayOutputStream output=new ByteArrayOutputStream()){byte[] bytes=new byte[4096];int count;while((count=input.read(bytes))!=-1){if(output.size()+count>1024*1024)throw new IOException("Engine response too large");output.write(bytes,0,count);}return new String(output.toByteArray(),StandardCharsets.UTF_8);}}finally{connection.disconnect();}
    }
}
