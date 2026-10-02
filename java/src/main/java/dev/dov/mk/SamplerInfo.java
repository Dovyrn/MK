package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SamplerInfo {
    public int min = Filter.LINEAR;
    public int mag = Filter.LINEAR;
    public int mip = Filter.LINEAR;
    public int u = Address.REPEAT;
    public int v = Address.REPEAT;
    public int w = Address.REPEAT;
    public int anisotropy = 1;
    public int compare = Compare.NONE;
    public float minLod = 0.0f;
    public float maxLod = 1000.0f;
    public int border = Border.TRANSPARENT_BLACK;

    public static final int SIZE = 44;

    int encodedSize() {
        return 44;
    }

    long encode(long p) {
        memPutInt(p, min);
        p += 4;
        memPutInt(p, mag);
        p += 4;
        memPutInt(p, mip);
        p += 4;
        memPutInt(p, u);
        p += 4;
        memPutInt(p, v);
        p += 4;
        memPutInt(p, w);
        p += 4;
        memPutInt(p, anisotropy);
        p += 4;
        memPutInt(p, compare);
        p += 4;
        memPutFloat(p, minLod);
        p += 4;
        memPutFloat(p, maxLod);
        p += 4;
        memPutInt(p, border);
        p += 4;
        return p;
    }

    static SamplerInfo decode(long p) {
        var r = new SamplerInfo();
        r.min = memGetInt(p);
        p += 4;
        r.mag = memGetInt(p);
        p += 4;
        r.mip = memGetInt(p);
        p += 4;
        r.u = memGetInt(p);
        p += 4;
        r.v = memGetInt(p);
        p += 4;
        r.w = memGetInt(p);
        p += 4;
        r.anisotropy = memGetInt(p);
        p += 4;
        r.compare = memGetInt(p);
        p += 4;
        r.minLod = memGetFloat(p);
        p += 4;
        r.maxLod = memGetFloat(p);
        p += 4;
        r.border = memGetInt(p);
        p += 4;
        return r;
    }
}
