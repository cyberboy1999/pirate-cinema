package com.piratecinema.app;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.zip.Deflater;
import java.util.zip.DeflaterOutputStream;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(AndroidJUnit4.class)
public class TorrentSearchTest {
    @Test
    public void rawDeflateIndexIsFilteredAndRanked() throws Exception {
        String hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        String other = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        String json = "[{\"Title\":\"Матрица / The Matrix (1999) BDRip 1080p\",\"Name\":\"Матрица\",\"Names\":[\"The Matrix\"],\"Size\":\"10 GB\",\"CreateDate\":\"2026-01-01\",\"Tracker\":\"Rutor\",\"Magnet\":\"magnet:?xt=urn:btih:" + hash + "\",\"Hash\":\"" + hash + "\",\"Year\":1999,\"Peer\":2,\"Seed\":12,\"VideoQuality\":200},{\"Title\":\"Другой фильм\",\"Names\":null,\"Hash\":\"" + other + "\",\"Magnet\":\"magnet:?xt=urn:btih:" + other + "\"}]";
        File index = new File(InstrumentationRegistry.getInstrumentation().getTargetContext().getCacheDir(), "rutor-test.ls");
        Deflater deflater = new Deflater(Deflater.DEFAULT_COMPRESSION, true);
        try (DeflaterOutputStream output = new DeflaterOutputStream(new FileOutputStream(index), deflater)) { output.write(json.getBytes(StandardCharsets.UTF_8)); }
        try {
            var results = TorrentSearchPlugin.searchIndex(index, "the matrix 1999");
            assertEquals(1, results.length());
            assertEquals(hash, results.getJSONObject(0).getString("hash"));
            assertEquals("1080p", results.getJSONObject(0).getString("quality"));
            assertTrue(results.getJSONObject(0).getString("magnet").startsWith("magnet:?"));
            try { TorrentSearchPlugin.searchIndex(index, "--"); fail("punctuation-only query must be rejected"); }
            catch (IllegalStateException expected) { assertTrue(expected.getMessage().contains("letters or digits")); }
        } finally { index.delete(); deflater.end(); }
    }
}
