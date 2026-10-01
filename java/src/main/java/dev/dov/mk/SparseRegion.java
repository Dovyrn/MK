package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class SparseRegion {
    public int mip = 0;
    public int layer = 0;
    public int x = 0;
    public int y = 0;
    public int z = 0;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int heapPage = 0;

    public int size() {
        return 36;
    }

    public long write(long p) {
        memPutInt(p, mip);
        p += 4;
        memPutInt(p, layer);
        p += 4;
        memPutInt(p, x);
        p += 4;
        memPutInt(p, y);
        p += 4;
        memPutInt(p, z);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, heapPage);
        p += 4;
        return p;
    }

    public static SparseRegion read(long p) {
        var r = new SparseRegion();
        r.mip = memGetInt(p);
        p += 4;
        r.layer = memGetInt(p);
        p += 4;
        r.x = memGetInt(p);
        p += 4;
        r.y = memGetInt(p);
        p += 4;
        r.z = memGetInt(p);
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        r.heapPage = memGetInt(p);
        p += 4;
        return r;
    }
}
