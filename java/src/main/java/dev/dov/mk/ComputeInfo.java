package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ComputeInfo {
    public long code = 0;
    public long codeSize = 0;
    public String entry = "";
    public int[] group = new int[] {1, 1, 1};
    public String[] functions = new String[0];
    public boolean indirect = false;

    int encodedSize() {
        var functionsSize = 4;
        for (var x : functions) {
            functionsSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        return 36 + entry.getBytes(StandardCharsets.UTF_8).length + functionsSize;
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
        memPutInt(p, functions.length);
        p += 4;
        for (var x : functions) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, indirect ? 1 : 0);
        p += 4;
        return p;
    }
}
