package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VertexBuffer {
    public int stride = 0;
    public boolean instance = false;

    public int size() {
        return 8;
    }

    public long write(long p) {
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, instance ? 1 : 0);
        p += 4;
        return p;
    }

    public static VertexBuffer read(long p) {
        var r = new VertexBuffer();
        r.stride = memGetInt(p);
        p += 4;
        r.instance = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
