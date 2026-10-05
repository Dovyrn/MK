package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TileInfo {
    public long code = 0;
    public long codeSize = 0;
    public String entry = "";
    public ColorTarget[] colors = new ColorTarget[0];
    public int samples = 1;

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 24 + entry.getBytes(StandardCharsets.UTF_8).length + colorsSize;
    }

    long encode(long p) {
        memPutLong(p, code);
        memPutLong(p + 8, codeSize);
        p += 16;
        var entryBytes = entry.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, entryBytes.length);
        memByteBuffer(p + 4, entryBytes.length).put(entryBytes);
        p += 4 + entryBytes.length;
        memPutInt(p, colors.length);
        p += 4;
        for (var x : colors) {
            p = x.encode(p);
        }
        memPutInt(p, samples);
        p += 4;
        return p;
    }
}
