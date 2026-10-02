package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class Boxes {
    public int buffer = 0;
    public long offset = 0;
    public int stride = 24;
    public int count = 0;
    public boolean opaque = true;

    public static final int SIZE = 24;

    public int size() {
        return 24;
    }

    public long write(long p) {
        memPutInt(p, buffer);
        p += 4;
        memPutLong(p, offset);
        p += 8;
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, count);
        p += 4;
        memPutInt(p, opaque ? 1 : 0);
        p += 4;
        return p;
    }

    public static Boxes read(long p) {
        var r = new Boxes();
        r.buffer = memGetInt(p);
        p += 4;
        r.offset = memGetLong(p);
        p += 8;
        r.stride = memGetInt(p);
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        r.opaque = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
