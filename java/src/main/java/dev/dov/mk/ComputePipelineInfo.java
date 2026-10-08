package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ComputePipelineInfo {
    public long code = 0;
    public long codeSize = 0;
    public String entry = "";
    public int subgroup = 0;
    public boolean full = false;

    int encodedSize() {
        return 28 + entry.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, code);
        memPutLong(p + 8, codeSize);
        p += 16;
        var entryBytes = entry.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, entryBytes.length);
        memByteBuffer(p + 4, entryBytes.length).put(entryBytes);
        p += 4 + entryBytes.length;
        memPutInt(p, subgroup);
        p += 4;
        memPutInt(p, full ? 1 : 0);
        p += 4;
        return p;
    }
}
