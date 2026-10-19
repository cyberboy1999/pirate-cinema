package com.piratecinema.app;

import androidx.test.ext.junit.runners.AndroidJUnit4;
import com.arthenica.ffmpegkit.FFmpegKit;
import com.arthenica.ffmpegkit.ReturnCode;
import org.json.JSONObject;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

@RunWith(AndroidJUnit4.class)
public class TorrentMediaTest {
    @Test public void storageGuardHonorsReserveAndQuotaAndCountsSparseBlocks() throws Exception {
        assertEquals("",TorrentService.storageWarning(1,1024L*1024*1024,2));
        assertFalse(TorrentService.storageWarning(2,1024L*1024*1024,2).isEmpty());
        assertFalse(TorrentService.storageWarning(0,1,1024).isEmpty());
        java.io.File file=java.io.File.createTempFile("owned-sparse-test-",".bin",androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().getTargetContext().getCacheDir());
        try{try(java.io.RandomAccessFile stream=new java.io.RandomAccessFile(file,"rw")){stream.setLength(1024L*1024*1024);}assertTrue(TorrentService.allocated(file,file.getParentFile().getCanonicalPath()+java.io.File.separator)<1024*1024);}finally{assertTrue(file.delete());}
    }
    @Test public void nativeRuntimeLoadsWithItsDependencies(){
        var session=FFmpegKit.executeWithArguments(new String[]{"-version"});
        assertTrue(ReturnCode.isSuccess(session.getReturnCode()));
        System.out.println("Native media runtime: "+session.getOutput().split("\n")[0]);
    }
    @Test public void vodTimelineAndAudioVariantsAreValid() throws Exception {
        TorrentMediaPlugin.Media media=new TorrentMediaPlugin.Media();
        media.height=360;media.duration=13.25;
        media.audio.put(new JSONObject().put("name","1 · eng"));
        media.audio.put(new JSONObject().put("name","2 · eng\"\n"));
        String master=TorrentMediaPlugin.master(media);
        assertTrue(master.contains("NAME=\"1 · eng\""));
        assertTrue(master.contains("NAME=\"2 · eng\""));
        assertTrue(master.contains("video/240/index.m3u8"));
        assertTrue(master.contains("video/360/index.m3u8"));
        assertFalse(master.contains("video/480/"));
        String playlist=TorrentMediaPlugin.playlist(media);
        assertTrue(playlist.contains("#EXTINF:1.250,\nsegment-2.ts"));
        assertFalse(playlist.contains("segment-3.ts"));
        assertTrue(playlist.endsWith("#EXT-X-ENDLIST\n"));
    }
}
