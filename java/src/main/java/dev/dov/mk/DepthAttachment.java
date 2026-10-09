package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DepthAttachment {
    public Texture texture = null;
    public LoadOp load = LoadOp.CLEAR;
    public StoreOp store = StoreOp.STORE;
    public float clearDepth = 1.0f;
    public LoadOp stencilLoad = LoadOp.DONT_CARE;
    public StoreOp stencilStore = StoreOp.DONT_CARE;
    public int clearStencil = 0;

    public static final int SIZE = 32;

    int encodedSize() {
        return 32;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, load.value);
        p += 4;
        memPutInt(p, store.value);
        p += 4;
        memPutFloat(p, clearDepth);
        p += 4;
        memPutInt(p, stencilLoad.value);
        p += 4;
        memPutInt(p, stencilStore.value);
        p += 4;
        memPutInt(p, clearStencil);
        p += 4;
        return p;
    }

    static DepthAttachment decode(long p) {
        var r = new DepthAttachment();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.load = LoadOp.of(memGetInt(p));
        p += 4;
        r.store = StoreOp.of(memGetInt(p));
        p += 4;
        r.clearDepth = memGetFloat(p);
        p += 4;
        r.stencilLoad = LoadOp.of(memGetInt(p));
        p += 4;
        r.stencilStore = StoreOp.of(memGetInt(p));
        p += 4;
        r.clearStencil = memGetInt(p);
        p += 4;
        return r;
    }
}
