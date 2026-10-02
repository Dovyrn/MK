package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

public class ComputeInfo {
    public ByteBuffer code = null;
    public String entry = "";
    public int[] group = new int[] {1, 1, 1};
    public String[] functions = new String[0];
    public boolean indirect = false;

    public int size() {
        var functionsSize = 0;
        for (var x : functions) {
            functionsSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        return 40 + entry.getBytes(StandardCharsets.UTF_8).length + functionsSize;
    }

    public long write(long p) {
        if (code != null && !code.isDirect()) {
            throw new IllegalArgumentException("code must be a direct ByteBuffer");
        }
        memPutLong(p, code == null ? 0 : memAddress(code));
        memPutLong(p + 8, code == null ? 0 : code.remaining());
        p += 16;
        var entryBytes = entry.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, entryBytes.length);
        memByteBuffer(p + 4, entryBytes.length).put(entryBytes);
        p += 4 + entryBytes.length;
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
