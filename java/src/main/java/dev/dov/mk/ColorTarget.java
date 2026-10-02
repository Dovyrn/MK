package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class ColorTarget {
    public int format = Format.NONE;
    public boolean blend = false;
    public int src = Blend.ONE;
    public int dst = Blend.ZERO;
    public int op = Op.ADD;
    public int srcAlpha = Blend.ONE;
    public int dstAlpha = Blend.ZERO;
    public int opAlpha = Op.ADD;
    public int mask = Mask.ALL;

    public static final int SIZE = 36;

    public int size() {
        return 36;
    }

    public long write(long p) {
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

    public static ColorTarget read(long p) {
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
