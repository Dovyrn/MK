package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class StencilFace {
    public int fail = Stencil.KEEP;
    public int depthFail = Stencil.KEEP;
    public int pass = Stencil.KEEP;
    public int compare = Compare.ALWAYS;
    public int read = 0xff;
    public int write = 0xff;

    public int size() {
        return 24;
    }

    public long write(long p) {
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

    public static StencilFace read(long p) {
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
