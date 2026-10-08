package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorTarget {
    public Format format = Format.NONE;
    public boolean blend = false;
    public BlendFactor src = BlendFactor.ONE;
    public BlendFactor dst = BlendFactor.ZERO;
    public BlendOp op = BlendOp.ADD;
    public BlendFactor srcAlpha = BlendFactor.ONE;
    public BlendFactor dstAlpha = BlendFactor.ZERO;
    public BlendOp opAlpha = BlendOp.ADD;
    public int mask = ColorMask.ALL;

    public static final int SIZE = 36;

    int encodedSize() {
        return 36;
    }

    long encode(long p) {
        memPutInt(p, format.value);
        p += 4;
        memPutInt(p, blend ? 1 : 0);
        p += 4;
        memPutInt(p, src.value);
        p += 4;
        memPutInt(p, dst.value);
        p += 4;
        memPutInt(p, op.value);
        p += 4;
        memPutInt(p, srcAlpha.value);
        p += 4;
        memPutInt(p, dstAlpha.value);
        p += 4;
        memPutInt(p, opAlpha.value);
        p += 4;
        memPutInt(p, mask);
        p += 4;
        return p;
    }

    static ColorTarget decode(long p) {
        var r = new ColorTarget();
        r.format = Format.of(memGetInt(p));
        p += 4;
        r.blend = memGetInt(p) != 0;
        p += 4;
        r.src = BlendFactor.of(memGetInt(p));
        p += 4;
        r.dst = BlendFactor.of(memGetInt(p));
        p += 4;
        r.op = BlendOp.of(memGetInt(p));
        p += 4;
        r.srcAlpha = BlendFactor.of(memGetInt(p));
        p += 4;
        r.dstAlpha = BlendFactor.of(memGetInt(p));
        p += 4;
        r.opAlpha = BlendOp.of(memGetInt(p));
        p += 4;
        r.mask = memGetInt(p);
        p += 4;
        return r;
    }
}
