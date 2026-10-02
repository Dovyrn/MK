package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VertexAttribute {
    public int location = 0;
    public int buffer = 0;
    public int format = 0;
    public int offset = 0;

    public static final int SIZE = 16;

    public int size() {
        return 16;
    }

    public long write(long p) {
        memPutInt(p, location);
        p += 4;
        memPutInt(p, buffer);
        p += 4;
        memPutInt(p, format);
        p += 4;
        memPutInt(p, offset);
        p += 4;
        return p;
    }

    public static VertexAttribute read(long p) {
        var r = new VertexAttribute();
        r.location = memGetInt(p);
        p += 4;
        r.buffer = memGetInt(p);
        p += 4;
        r.format = memGetInt(p);
        p += 4;
        r.offset = memGetInt(p);
        p += 4;
        return r;
    }
}
