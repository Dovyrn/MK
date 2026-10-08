package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorAttachment {
    public Texture texture = null;
    public int load = LoadOp.CLEAR;
    public int store = StoreOp.STORE;
    public float r = 0.0f;
    public float g = 0.0f;
    public float b = 0.0f;
    public float a = 0.0f;
    public Texture resolve = null;

    public static final int SIZE = 40;

    int encodedSize() {
        return 40;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, load);
        p += 4;
        memPutInt(p, store);
        p += 4;
        memPutFloat(p, r);
        p += 4;
        memPutFloat(p, g);
        p += 4;
        memPutFloat(p, b);
        p += 4;
        memPutFloat(p, a);
        p += 4;
        memPutLong(p, Texture.raw(resolve));
        p += 8;
        return p;
    }

    static ColorAttachment decode(long p) {
        var r = new ColorAttachment();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.load = memGetInt(p);
        p += 4;
        r.store = memGetInt(p);
        p += 4;
        r.r = memGetFloat(p);
        p += 4;
        r.g = memGetFloat(p);
        p += 4;
        r.b = memGetFloat(p);
        p += 4;
        r.a = memGetFloat(p);
        p += 4;
        r.resolve = Texture.of(memGetLong(p));
        p += 8;
        return r;
    }
}
