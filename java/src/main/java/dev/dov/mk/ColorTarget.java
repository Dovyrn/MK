package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorTarget {
    public Format format = Format.NONE;
    public boolean blend = false;
    public BlendFactor srcColor = BlendFactor.ONE;
    public BlendFactor dstColor = BlendFactor.ZERO;
    public BlendOp opColor = BlendOp.ADD;
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
        memPutInt(p, srcColor.value);
        p += 4;
        memPutInt(p, dstColor.value);
        p += 4;
        memPutInt(p, opColor.value);
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
        r.srcColor = BlendFactor.of(memGetInt(p));
        p += 4;
        r.dstColor = BlendFactor.of(memGetInt(p));
        p += 4;
        r.opColor = BlendOp.of(memGetInt(p));
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
