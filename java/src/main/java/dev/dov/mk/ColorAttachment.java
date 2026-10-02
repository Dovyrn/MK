package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorAttachment {
    public int index = 0;
    public int texture = 0;
    public int layout = Layout.COLOR;
    public int load = Load.CLEAR;
    public int store = Store.STORE;
    public float r = 0.0f;
    public float g = 0.0f;
    public float b = 0.0f;
    public float a = 0.0f;
    public int resolve = -1;
    public int resolveLayout = Layout.COLOR;

    public static final int SIZE = 44;

    int encodedSize() {
        return 44;
    }

    long encode(long p) {
        memPutInt(p, index);
        p += 4;
        memPutInt(p, texture);
        p += 4;
        memPutInt(p, layout);
        p += 4;
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
        memPutInt(p, resolve);
        p += 4;
        memPutInt(p, resolveLayout);
        p += 4;
        return p;
    }

    static ColorAttachment decode(long p) {
        var r = new ColorAttachment();
        r.index = memGetInt(p);
        p += 4;
        r.texture = memGetInt(p);
        p += 4;
        r.layout = memGetInt(p);
        p += 4;
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
        r.resolve = memGetInt(p);
        p += 4;
        r.resolveLayout = memGetInt(p);
        p += 4;
        return r;
    }
}
