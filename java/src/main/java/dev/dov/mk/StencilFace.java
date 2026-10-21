package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class StencilFace {
    public StencilOp failOp = StencilOp.KEEP;
    public StencilOp depthFailOp = StencilOp.KEEP;
    public StencilOp passOp = StencilOp.KEEP;
    public CompareOp compare = CompareOp.ALWAYS;
    public int readMask = 0xff;
    public int writeMask = 0xff;

    public static final int SIZE = 24;

    int encodedSize() {
        return 24;
    }

    long encode(long p) {
        memPutInt(p, failOp.value);
        p += 4;
        memPutInt(p, depthFailOp.value);
        p += 4;
        memPutInt(p, passOp.value);
        p += 4;
        memPutInt(p, compare.value);
        p += 4;
        memPutInt(p, readMask);
        p += 4;
        memPutInt(p, writeMask);
        p += 4;
        return p;
    }

    static StencilFace decode(long p) {
        var r = new StencilFace();
        r.failOp = StencilOp.of(memGetInt(p));
        p += 4;
        r.depthFailOp = StencilOp.of(memGetInt(p));
        p += 4;
        r.passOp = StencilOp.of(memGetInt(p));
        p += 4;
        r.compare = CompareOp.of(memGetInt(p));
        p += 4;
        r.readMask = memGetInt(p);
        p += 4;
        r.writeMask = memGetInt(p);
        p += 4;
        return r;
    }
}
