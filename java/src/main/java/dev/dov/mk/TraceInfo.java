package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class TraceInfo {
    public int raygen = 0;
    public long raygenOffset = 0;
    public long raygenStride = 0;
    public long raygenSize = 0;
    public int miss = 0;
    public long missOffset = 0;
    public long missStride = 0;
    public long missSize = 0;
    public int hit = 0;
    public long hitOffset = 0;
    public long hitStride = 0;
    public long hitSize = 0;
    public int callable = 0;
    public long callableOffset = 0;
    public long callableStride = 0;
    public long callableSize = 0;
    public int width = 1;
    public int height = 1;
    public int depth = 1;

    public int size() {
        return 124;
    }

    public long write(long p) {
        memPutInt(p, raygen);
        p += 4;
        memPutLong(p, raygenOffset);
        p += 8;
        memPutLong(p, raygenStride);
        p += 8;
        memPutLong(p, raygenSize);
        p += 8;
        memPutInt(p, miss);
        p += 4;
        memPutLong(p, missOffset);
        p += 8;
        memPutLong(p, missStride);
        p += 8;
        memPutLong(p, missSize);
        p += 8;
        memPutInt(p, hit);
        p += 4;
        memPutLong(p, hitOffset);
        p += 8;
        memPutLong(p, hitStride);
        p += 8;
        memPutLong(p, hitSize);
        p += 8;
        memPutInt(p, callable);
        p += 4;
        memPutLong(p, callableOffset);
        p += 8;
        memPutLong(p, callableStride);
        p += 8;
        memPutLong(p, callableSize);
        p += 8;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        return p;
    }

    public static TraceInfo read(long p) {
        var r = new TraceInfo();
        r.raygen = memGetInt(p);
        p += 4;
        r.raygenOffset = memGetLong(p);
        p += 8;
        r.raygenStride = memGetLong(p);
        p += 8;
        r.raygenSize = memGetLong(p);
        p += 8;
        r.miss = memGetInt(p);
        p += 4;
        r.missOffset = memGetLong(p);
        p += 8;
        r.missStride = memGetLong(p);
        p += 8;
        r.missSize = memGetLong(p);
        p += 8;
        r.hit = memGetInt(p);
        p += 4;
        r.hitOffset = memGetLong(p);
        p += 8;
        r.hitStride = memGetLong(p);
        p += 8;
        r.hitSize = memGetLong(p);
        p += 8;
        r.callable = memGetInt(p);
        p += 4;
        r.callableOffset = memGetLong(p);
        p += 8;
        r.callableStride = memGetLong(p);
        p += 8;
        r.callableSize = memGetLong(p);
        p += 8;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        return r;
    }
}
