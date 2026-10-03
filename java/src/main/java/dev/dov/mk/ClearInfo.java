package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ClearInfo {
    public int texture = 0;
    public int layout = Layout.COPY_DST;
    public int baseMip = 0;
    public int mips = -1;
    public int baseLayer = 0;
    public int layers = -1;
    public float r = 0.0f;
    public float g = 0.0f;
    public float b = 0.0f;
    public float a = 0.0f;
    public float depth = 1.0f;
    public int stencil = 0;

    public static final int SIZE = 48;

    int encodedSize() {
        return 48;
    }

    long encode(long p) {
        memPutInt(p, texture);
        p += 4;
        memPutInt(p, layout);
        p += 4;
        memPutInt(p, baseMip);
        p += 4;
        memPutInt(p, mips);
        p += 4;
        memPutInt(p, baseLayer);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        memPutFloat(p, r);
        p += 4;
        memPutFloat(p, g);
        p += 4;
        memPutFloat(p, b);
        p += 4;
        memPutFloat(p, a);
        p += 4;
        memPutFloat(p, depth);
        p += 4;
        memPutInt(p, stencil);
        p += 4;
        return p;
    }

    static ClearInfo decode(long p) {
        var r = new ClearInfo();
        r.texture = memGetInt(p);
        p += 4;
        r.layout = memGetInt(p);
        p += 4;
        r.baseMip = memGetInt(p);
        p += 4;
        r.mips = memGetInt(p);
        p += 4;
        r.baseLayer = memGetInt(p);
        p += 4;
        r.layers = memGetInt(p);
        p += 4;
        r.r = memGetFloat(p);
        p += 4;
        r.g = memGetFloat(p);
        p += 4;
        r.b = memGetFloat(p);
        p += 4;
        r.a = memGetFloat(p);
        p += 4;
        r.depth = memGetFloat(p);
        p += 4;
        r.stencil = memGetInt(p);
        p += 4;
        return r;
    }
}
