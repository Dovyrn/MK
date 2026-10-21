package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DepthStencilStateInfo {
    public CompareOp compare = CompareOp.NONE;
    public boolean write = false;
    public boolean stencilTest = false;
    public StencilFace front = new StencilFace();
    public StencilFace back = new StencilFace();

    public static final int SIZE = 12 + StencilFace.SIZE + StencilFace.SIZE;

    int encodedSize() {
        return 12 + front.encodedSize() + back.encodedSize();
    }

    long encode(long p) {
        memPutInt(p, compare.value);
        p += 4;
        memPutInt(p, write ? 1 : 0);
        p += 4;
        memPutInt(p, stencilTest ? 1 : 0);
        p += 4;
        p = front.encode(p);
        p = back.encode(p);
        return p;
    }

    static DepthStencilStateInfo decode(long p) {
        var r = new DepthStencilStateInfo();
        r.compare = CompareOp.of(memGetInt(p));
        p += 4;
        r.write = memGetInt(p) != 0;
        p += 4;
        r.stencilTest = memGetInt(p) != 0;
        p += 4;
        r.front = StencilFace.decode(p);
        p += r.front.encodedSize();
        r.back = StencilFace.decode(p);
        p += r.back.encodedSize();
        return r;
    }
}
