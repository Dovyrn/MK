package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class HostCopy {
    public int texture = 0;
    public int layout = Layout.GENERAL;
    public int mip = 0;
    public int layer = 0;
    public int x = 0;
    public int y = 0;
    public int z = 0;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int rowLength = 0;
    public int rowCount = 0;

    public static final int SIZE = 48;

    int encodedSize() {
        return 48;
    }

    long encode(long p) {
        memPutInt(p, texture);
        p += 4;
        memPutInt(p, layout);
        p += 4;
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
        memPutInt(p, rowLength);
        p += 4;
        memPutInt(p, rowCount);
        p += 4;
        return p;
    }

    static HostCopy decode(long p) {
        var r = new HostCopy();
        r.texture = memGetInt(p);
        p += 4;
        r.layout = memGetInt(p);
        p += 4;
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
        r.rowLength = memGetInt(p);
        p += 4;
        r.rowCount = memGetInt(p);
        p += 4;
        return r;
    }
}
