package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class StencilFace {
    public StencilOp fail = StencilOp.KEEP;
    public StencilOp depthFail = StencilOp.KEEP;
    public StencilOp pass = StencilOp.KEEP;
    public CompareOp compare = CompareOp.ALWAYS;
    public int read = 0xff;
    public int write = 0xff;

    public static final int SIZE = 24;

    int encodedSize() {
        return 24;
    }

    long encode(long p) {
        memPutInt(p, fail.value);
        p += 4;
        memPutInt(p, depthFail.value);
        p += 4;
        memPutInt(p, pass.value);
        p += 4;
        memPutInt(p, compare.value);
        p += 4;
        memPutInt(p, read);
        p += 4;
        memPutInt(p, write);
        p += 4;
        return p;
    }

    static StencilFace decode(long p) {
        var r = new StencilFace();
        r.fail = StencilOp.of(memGetInt(p));
        p += 4;
        r.depthFail = StencilOp.of(memGetInt(p));
        p += 4;
        r.pass = StencilOp.of(memGetInt(p));
        p += 4;
        r.compare = CompareOp.of(memGetInt(p));
        p += 4;
        r.read = memGetInt(p);
        p += 4;
        r.write = memGetInt(p);
        p += 4;
        return r;
    }
}
