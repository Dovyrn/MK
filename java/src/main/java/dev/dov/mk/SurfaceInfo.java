package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class SurfaceInfo {
    public int width = 1;
    public int height = 1;
    public int format = Format.BGRA8_UNORM;
    public boolean vsync = true;
    public int images = 3;
    public int usage = Usage.COLOR;

    public int size() {
        return 24;
    }

    public long write(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, format);
        p += 4;
        memPutInt(p, vsync ? 1 : 0);
        p += 4;
        memPutInt(p, images);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        return p;
    }

    public static SurfaceInfo read(long p) {
        var r = new SurfaceInfo();
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.format = memGetInt(p);
        p += 4;
        r.vsync = memGetInt(p) != 0;
        p += 4;
        r.images = memGetInt(p);
        p += 4;
        r.usage = memGetInt(p);
        p += 4;
        return r;
    }
}
