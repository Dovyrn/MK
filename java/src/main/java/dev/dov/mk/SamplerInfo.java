package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SamplerInfo {
    public Filter min = Filter.LINEAR;
    public Filter mag = Filter.LINEAR;
    public Filter mip = Filter.LINEAR;
    public AddressMode u = AddressMode.REPEAT;
    public AddressMode v = AddressMode.REPEAT;
    public AddressMode w = AddressMode.REPEAT;
    public int anisotropy = 1;
    public CompareOp compare = CompareOp.NONE;
    public float minLod = 0.0f;
    public float maxLod = 1000.0f;
    public BorderColor border = BorderColor.TRANSPARENT_BLACK;

    public static final int SIZE = 44;

    int encodedSize() {
        return 44;
    }

    long encode(long p) {
        memPutInt(p, min.value);
        p += 4;
        memPutInt(p, mag.value);
        p += 4;
        memPutInt(p, mip.value);
        p += 4;
        memPutInt(p, u.value);
        p += 4;
        memPutInt(p, v.value);
        p += 4;
        memPutInt(p, w.value);
        p += 4;
        memPutInt(p, anisotropy);
        p += 4;
        memPutInt(p, compare.value);
        p += 4;
        memPutFloat(p, minLod);
        p += 4;
        memPutFloat(p, maxLod);
        p += 4;
        memPutInt(p, border.value);
        p += 4;
        return p;
    }

    static SamplerInfo decode(long p) {
        var r = new SamplerInfo();
        r.min = Filter.of(memGetInt(p));
        p += 4;
        r.mag = Filter.of(memGetInt(p));
        p += 4;
        r.mip = Filter.of(memGetInt(p));
        p += 4;
        r.u = AddressMode.of(memGetInt(p));
        p += 4;
        r.v = AddressMode.of(memGetInt(p));
        p += 4;
        r.w = AddressMode.of(memGetInt(p));
        p += 4;
        r.anisotropy = memGetInt(p);
        p += 4;
        r.compare = CompareOp.of(memGetInt(p));
        p += 4;
        r.minLod = memGetFloat(p);
        p += 4;
        r.maxLod = memGetFloat(p);
        p += 4;
        r.border = BorderColor.of(memGetInt(p));
        p += 4;
        return r;
    }
}
