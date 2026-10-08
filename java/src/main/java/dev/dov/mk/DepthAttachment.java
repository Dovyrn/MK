package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DepthAttachment {
    public Texture texture = null;
    public int load = LoadOp.CLEAR;
    public int store = StoreOp.STORE;
    public float depth = 1.0f;
    public int stencilLoad = LoadOp.DONT_CARE;
    public int stencilStore = StoreOp.DONT_CARE;
    public int stencil = 0;

    public static final int SIZE = 32;

    int encodedSize() {
        return 32;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
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

    static DepthAttachment decode(long p) {
        var r = new DepthAttachment();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
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
