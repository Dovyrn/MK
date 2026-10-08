package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorTarget {
    public int format = Format.NONE;
    public boolean blend = false;
    public int src = BlendFactor.ONE;
    public int dst = BlendFactor.ZERO;
    public int op = BlendOp.ADD;
    public int srcAlpha = BlendFactor.ONE;
    public int dstAlpha = BlendFactor.ZERO;
    public int opAlpha = BlendOp.ADD;
    public int mask = ColorMask.ALL;

    public static final int SIZE = 36;

    int encodedSize() {
        return 36;
    }

    long encode(long p) {
        memPutInt(p, format);
        p += 4;
        memPutInt(p, blend ? 1 : 0);
        p += 4;
        memPutInt(p, src);
        p += 4;
        memPutInt(p, dst);
        p += 4;
        memPutInt(p, op);
        p += 4;
        memPutInt(p, srcAlpha);
        p += 4;
        memPutInt(p, dstAlpha);
        p += 4;
        memPutInt(p, opAlpha);
        p += 4;
        memPutInt(p, mask);
        p += 4;
        return p;
    }

    static ColorTarget decode(long p) {
        var r = new ColorTarget();
        r.format = memGetInt(p);
        p += 4;
        r.blend = memGetInt(p) != 0;
        p += 4;
        r.src = memGetInt(p);
        p += 4;
        r.dst = memGetInt(p);
        p += 4;
        r.op = memGetInt(p);
        p += 4;
        r.srcAlpha = memGetInt(p);
        p += 4;
        r.dstAlpha = memGetInt(p);
        p += 4;
        r.opAlpha = memGetInt(p);
        p += 4;
        r.mask = memGetInt(p);
        p += 4;
        return r;
    }
}
