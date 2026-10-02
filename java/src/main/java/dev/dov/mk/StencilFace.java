package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class StencilFace {
    public int fail = Stencil.KEEP;
    public int depthFail = Stencil.KEEP;
    public int pass = Stencil.KEEP;
    public int compare = Compare.ALWAYS;
    public int read = 0xff;
    public int write = 0xff;

    public static final int SIZE = 24;

    int encodedSize() {
        return 24;
    }

    long encode(long p) {
        memPutInt(p, fail);
        p += 4;
        memPutInt(p, depthFail);
        p += 4;
        memPutInt(p, pass);
        p += 4;
        memPutInt(p, compare);
        p += 4;
        memPutInt(p, read);
        p += 4;
        memPutInt(p, write);
        p += 4;
        return p;
    }

    static StencilFace decode(long p) {
        var r = new StencilFace();
        r.fail = memGetInt(p);
        p += 4;
        r.depthFail = memGetInt(p);
        p += 4;
        r.pass = memGetInt(p);
        p += 4;
        r.compare = memGetInt(p);
        p += 4;
        r.read = memGetInt(p);
        p += 4;
        r.write = memGetInt(p);
        p += 4;
        return r;
    }
}
