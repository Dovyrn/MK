package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class DepthAttachment {
    public int texture = 0;
    public int layout = Layout.DEPTH;
    public int load = Load.CLEAR;
    public int store = Store.STORE;
    public float depth = 1.0f;
    public int stencilLoad = Load.DONT_CARE;
    public int stencilStore = Store.DONT_CARE;
    public int stencil = 0;

    public static final int SIZE = 32;

    public int size() {
        return 32;
    }

    public long write(long p) {
        memPutInt(p, texture);
        p += 4;
        memPutInt(p, layout);
        p += 4;
        memPutInt(p, load);
        p += 4;
        memPutInt(p, store);
        p += 4;
        memPutFloat(p, depth);
        p += 4;
        memPutInt(p, stencilLoad);
        p += 4;
        memPutInt(p, stencilStore);
        p += 4;
        memPutInt(p, stencil);
        p += 4;
        return p;
    }

    public static DepthAttachment read(long p) {
        var r = new DepthAttachment();
        r.texture = memGetInt(p);
        p += 4;
        r.layout = memGetInt(p);
        p += 4;
        r.load = memGetInt(p);
        p += 4;
        r.store = memGetInt(p);
        p += 4;
        r.depth = memGetFloat(p);
        p += 4;
        r.stencilLoad = memGetInt(p);
        p += 4;
        r.stencilStore = memGetInt(p);
        p += 4;
        r.stencil = memGetInt(p);
        p += 4;
        return r;
    }
}
