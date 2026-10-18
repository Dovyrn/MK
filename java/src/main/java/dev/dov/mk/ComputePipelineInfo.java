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
    public int[] group = new int[] {0, 0, 0};
    public int subgroup = 0;
    public boolean full = false;
    public String label = "";

    int encodedSize() {
        return 44 + entry.getBytes(StandardCharsets.UTF_8).length + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, code);
        memPutLong(p + 8, codeSize);
        p += 16;
        var entryBytes = entry.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, entryBytes.length);
        memByteBuffer(p + 4, entryBytes.length).put(entryBytes);
        p += 4 + entryBytes.length;
        if (group.length != 3) {
            throw new IllegalArgumentException("group has 3 values");
        }
        for (var x : group) {
            memPutInt(p, x);
            p += 4;
        }
        memPutInt(p, subgroup);
        p += 4;
        memPutInt(p, full ? 1 : 0);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
