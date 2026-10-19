package com.piratecinema.app;

import com.arthenica.ffmpegkit.FFmpegKit;
import com.arthenica.ffmpegkit.FFprobeKit;
import com.arthenica.ffmpegkit.ReturnCode;
import com.getcapacitor.*;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;
import org.json.*;

/** On-demand local HLS: every segment reads verified pieces through librqbit. */
@CapacitorPlugin(name="TorrentMedia")
public class TorrentMediaPlugin extends Plugin {
    private final ExecutorService commands=Executors.newSingleThreadExecutor();
    private final Set<Long> sessions=ConcurrentHashMap.newKeySet();
    private final Set<String> pending=ConcurrentHashMap.newKeySet(),cancelled=ConcurrentHashMap.newKeySet();
    private volatile String probingToken="";
    private volatile long probeId;
    private final ThreadPoolExecutor requests=new ThreadPoolExecutor(2,2,0,TimeUnit.SECONDS,new ArrayBlockingQueue<>(8));
    private volatile ServerSocket server;
    private volatile Media media;
    static class Media {
        String source,token;
        double duration;
        int height;
        JSONArray audio=new JSONArray();
        File directory;
        volatile boolean closed;
    }

    @PluginMethod public void prepare(PluginCall call){
        String token=call.getString("token","");
        if(!token.matches("[a-f0-9]{32}")){call.reject("Invalid playback token");return;}
        pending.add(token);commands.execute(()->{
        try {
            if(cancelled.contains(token))throw new IOException("Подготовка отменена");
            String source=call.getString("url","");
            // Only our embedded engine is allowed: never a generic HTTP proxy.
            if(!source.matches("http://127\\.0\\.0\\.1:3030/torrents/[a-f0-9]{40}/stream/\\d+(?:/[^?#]*)?"))throw new IOException("Недопустимый адрес torrent stream");
            stopMedia();
            sweepCache();
            probingToken=token;CountDownLatch probed=new CountDownLatch(1);
            var probe=FFprobeKit.executeWithArgumentsAsync(new String[]{"-v","error","-rw_timeout","30000000","-show_streams","-show_format","-of","json",source},finished->probed.countDown());
            probeId=probe.getSessionId();
            if(cancelled.contains(token))FFmpegKit.cancel(probeId);
            if(!probed.await(45,TimeUnit.SECONDS)||cancelled.contains(token)){FFmpegKit.cancel(probeId);throw new IOException("Подготовка отменена или превышено время ожидания");}
            if(!ReturnCode.isSuccess(probe.getReturnCode()))throw new IOException("Не удалось прочитать дорожки torrent stream");
            JSONObject info=new JSONObject(probe.getOutput());
            Media next=new Media();next.source=source;next.token=token;
            next.duration=info.getJSONObject("format").optDouble("duration",0);
            // AAC padding can extend the container beyond the final video frame.
            next.duration=Math.floor(next.duration*24)/24;
            JSONArray streams=info.getJSONArray("streams");int audioIndex=0;
            for(int i=0;i<streams.length();i++){
                JSONObject stream=streams.getJSONObject(i);
                if("video".equals(stream.optString("codec_type"))&&next.height==0)next.height=stream.optInt("height");
                if("audio".equals(stream.optString("codec_type"))){
                    JSONObject tags=stream.optJSONObject("tags");
                    int number=++audioIndex;
                    next.audio.put(new JSONObject().put("index",number-1).put("name",number+" · "+(tags==null?"Audio":tags.optString("title",tags.optString("language","Audio")))).put("codec",stream.optString("codec_name")));
                }
            }
            if(!(next.duration>0&&next.duration<=7*24*3600)||next.height<=0)throw new IOException("Видео не имеет допустимой длительности или видеодорожки");
            next.directory=new File(getContext().getCacheDir(),"torrent-hls-"+next.token);
            if(!next.directory.mkdir())throw new IOException("Не удалось создать HLS кеш");
            media=next;ensureServer();
            JSObject result=new JSObject();result.put("url","http://127.0.0.1:"+server.getLocalPort()+"/"+next.token+"/master.m3u8");result.put("token",next.token);result.put("duration",next.duration);result.put("audio",next.audio);call.resolve(result);
        }catch(Exception|LinkageError error){android.util.Log.e("TorrentMedia","prepare failed",error);call.reject(error.getMessage());}
        finally{probingToken="";probeId=0;pending.remove(token);cancelled.remove(token);}
    });}

    @PluginMethod public void stop(PluginCall call){String token=call.getString("token","");if(pending.contains(token))cancelled.add(token);if(token.equals(probingToken)&&probeId!=0)FFmpegKit.cancel(probeId);commands.execute(()->{Media current=media;if(current!=null&&current.token.equals(token))stopMedia();call.resolve();});}

    private void sweepCache(){File[] dirs=getContext().getCacheDir().listFiles(file->file.isDirectory()&&file.getName().matches("torrent-hls-[a-f0-9]{32}"));if(dirs!=null)for(File dir:dirs){File[] files=dir.listFiles();if(files!=null)for(File file:files)if(file.isFile())file.delete();dir.delete();}}

    private synchronized void ensureServer() throws IOException {
        if(server!=null&&!server.isClosed())return;
        server=new ServerSocket(0,8,InetAddress.getByName("127.0.0.1"));
        Thread accept=new Thread(()->{while(!server.isClosed())try{
            Socket socket=server.accept();socket.setSoTimeout(10000);
            try{requests.execute(()->serve(socket));}catch(RejectedExecutionException full){socket.close();}
        }catch(IOException error){break;}},"torrent-hls-http");accept.setDaemon(true);accept.start();
    }

    private void serve(Socket socket){try(socket){
        BufferedReader reader=new BufferedReader(new InputStreamReader(socket.getInputStream(),StandardCharsets.US_ASCII));
        String line=boundedLine(reader,2048);if(line==null)return;
        String[] request=line.split(" ");if(request.length!=3)return;
        int total=0;while((line=boundedLine(reader,2048))!=null&&!line.isEmpty()){total+=line.length();if(total>16384)return;}
        OutputStream output=socket.getOutputStream();boolean head="HEAD".equals(request[0]);
        if(!head&&!"GET".equals(request[0])){respond(output,405,"text/plain",new byte[0],head);return;}
        Media current=media;
        if(current==null||current.closed||!request[1].startsWith("/"+current.token+"/")){respond(output,404,"text/plain",new byte[0],head);return;}
        String route=request[1].substring(current.token.length()+2);
        if("master.m3u8".equals(route)){respond(output,200,"application/vnd.apple.mpegurl",master(current).getBytes(StandardCharsets.UTF_8),head);return;}
        var match=java.util.regex.Pattern.compile("(video|audio)/(\\d+)/(index\\.m3u8|segment-(\\d+)\\.ts)").matcher(route);
        if(!match.matches()){respond(output,404,"text/plain",new byte[0],head);return;}
        boolean audio="audio".equals(match.group(1));int track=Integer.parseInt(match.group(2));
        if(audio?track>=current.audio.length():!heights(current).contains(track)){respond(output,404,"text/plain",new byte[0],head);return;}
        if("index.m3u8".equals(match.group(3))){respond(output,200,"application/vnd.apple.mpegurl",playlist(current).getBytes(StandardCharsets.UTF_8),head);return;}
        int segment=Integer.parseInt(match.group(4));if(segment<0||segment*6>=current.duration){respond(output,404,"text/plain",new byte[0],head);return;}
        File file;try{file=segment(current,audio,track,segment);}catch(Exception error){android.util.Log.w("TorrentMedia","segment failed: "+error.getMessage());respond(output,503,"text/plain",new byte[0],head);return;}
        headers(output,200,"video/mp2t",file.length());
        if(!head)try(InputStream input=new FileInputStream(file)){byte[] buffer=new byte[16384];int count;while((count=input.read(buffer))>=0)output.write(buffer,0,count);}
    }catch(Exception error){android.util.Log.w("TorrentMedia","request failed: "+error.getMessage());}}

    private static String boundedLine(Reader reader,int limit)throws IOException{StringBuilder line=new StringBuilder();int value;while((value=reader.read())!=-1){if(value=='\n')return line.toString().replace("\r","");if(line.length()>=limit)throw new IOException("HTTP line too long");line.append((char)value);}return line.length()==0?null:line.toString();}

    private static List<Integer> heights(Media media){List<Integer> result=new ArrayList<>();for(int height:new int[]{240,360,480,720,1080})if(height<media.height)result.add(height);result.add(media.height);return result;}
    static String master(Media media){
        StringBuilder result=new StringBuilder("#EXTM3U\n#EXT-X-VERSION:3\n");
        for(int i=0;i<media.audio.length();i++){
            String name=media.audio.optJSONObject(i).optString("name").replaceAll("[\\\"\\r\\n]","");
            result.append("#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"").append(name).append("\",DEFAULT=").append(i==0?"YES":"NO").append(",AUTOSELECT=YES,URI=\"audio/").append(i).append("/index.m3u8\"\n");
        }
        for(int height:heights(media))result.append("#EXT-X-STREAM-INF:BANDWIDTH=").append(Math.max(300000,height*2500)).append(",RESOLUTION=").append(height*16/9/2*2).append('x').append(height).append(media.audio.length()>0?",AUDIO=\"audio\"":"").append("\nvideo/").append(height).append("/index.m3u8\n");
        return result.toString();
    }
    static String playlist(Media media){StringBuilder result=new StringBuilder("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n");for(int i=0;i*6<media.duration;i++)result.append("#EXTINF:").append(String.format(Locale.US,"%.3f",Math.min(6,media.duration-i*6))).append(",\nsegment-").append(i).append(".ts\n");return result.append("#EXT-X-ENDLIST\n").toString();}

    private File segment(Media current,boolean audio,int track,int index)throws Exception{
        String name=(audio?"a":"v")+track+"-"+index+".ts";File target=new File(current.directory,name),partial=new File(current.directory,name+".part");
        // ponytail: serialize segment encoding to bound CPU/RAM on phones; parallelize only after profiling.
        synchronized(current){
            if(current.closed)throw new IOException("Поток закрыт");if(target.isFile())return target;
            File[] cached=current.directory.listFiles();long used=0;if(cached!=null)for(File file:cached)used+=file.length();
            if(used>64L*1024*1024&&cached!=null){Arrays.sort(cached,Comparator.comparingLong(File::lastModified));for(File file:cached){used-=file.length();file.delete();if(used<32L*1024*1024)break;}}
            if(current.directory.getUsableSpace()<32L*1024*1024)throw new IOException("Недостаточно свободного места для HLS");
            List<String> args=new ArrayList<>(Arrays.asList("-hide_banner","-loglevel","error","-y","-rw_timeout","30000000","-ss",Integer.toString(index*6),"-i",current.source,"-t","6","-threads","2"));
            if(audio)args.addAll(Arrays.asList("-map","0:a:"+track,"-vn","-c:a","aac","-ac","2","-ar","48000","-b:a","128k"));
            else args.addAll(Arrays.asList("-map","0:v:0","-an","-vf","scale=-2:"+track,"-c:v","libx264","-preset","ultrafast","-profile:v","baseline","-pix_fmt","yuv420p","-r","24","-g","48","-sc_threshold","0"));
            args.addAll(Arrays.asList("-output_ts_offset",Integer.toString(index*6),"-muxdelay","0","-f","mpegts",partial.getAbsolutePath()));
            CountDownLatch done=new CountDownLatch(1);var session=FFmpegKit.executeWithArgumentsAsync(args.toArray(new String[0]),finished->done.countDown());sessions.add(session.getSessionId());
            try{if(!done.await(90,TimeUnit.SECONDS)||current.closed){FFmpegKit.cancel(session.getSessionId());done.await(5,TimeUnit.SECONDS);throw new IOException("Отмена или таймаут подготовки сегмента");}if(!ReturnCode.isSuccess(session.getReturnCode())||partial.length()==0)throw new IOException("Ошибка кодирования сегмента: "+session.getOutput());if(!partial.renameTo(target))throw new IOException("Не удалось сохранить сегмент");}finally{partial.delete();sessions.remove(session.getSessionId());}
            return target;
        }
    }
    private static void headers(OutputStream output,int status,String type,long length)throws IOException{output.write(("HTTP/1.1 "+status+"\r\nContent-Type: "+type+"\r\nContent-Length: "+length+"\r\nAccess-Control-Allow-Origin: https://localhost\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").getBytes(StandardCharsets.US_ASCII));}
    private static void respond(OutputStream output,int status,String type,byte[] data,boolean head)throws IOException{headers(output,status,type,data.length);if(!head)output.write(data);}
    private void stopMedia(){Media previous=media;media=null;if(previous!=null){previous.closed=true;for(Long id:sessions)FFmpegKit.cancel(id);synchronized(previous){File[] files=previous.directory.listFiles();if(files!=null)for(File file:files)file.delete();previous.directory.delete();}}}
    @Override protected void handleOnDestroy(){cancelled.addAll(pending);if(probeId!=0)FFmpegKit.cancel(probeId);stopMedia();try{if(server!=null)server.close();}catch(IOException ignored){}requests.shutdownNow();commands.shutdownNow();}
}
