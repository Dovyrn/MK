package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class DepthInfo {
    public int compare = Compare.NONE;
    public boolean write = false;
    public boolean stencil = false;
    public StencilFace front = new StencilFace();
    public StencilFace back = new StencilFace();

    public int size() {
        return 12 + front.size() + back.size();
    }

    public long write(long p) {
        memPutInt(p, compare);
        p += 4;
        memPutInt(p, write ? 1 : 0);
        p += 4;
        memPutInt(p, stencil ? 1 : 0);
        p += 4;
        p = front.write(p);
        p = back.write(p);
        return p;
    }

    public static DepthInfo read(long p) {
        var r = new DepthInfo();
        r.compare = memGetInt(p);
        p += 4;
        r.write = memGetInt(p) != 0;
        p += 4;
        r.stencil = memGetInt(p) != 0;
        p += 4;
        r.front = StencilFace.read(p);
        p += r.front.size();
        r.back = StencilFace.read(p);
        p += r.back.size();
        return r;
    }
}
