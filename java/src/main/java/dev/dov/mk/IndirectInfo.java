package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class IndirectInfo {
    public int kinds = Indirect.DRAW;
    public int count = 1;
    public boolean inheritPipeline = true;
    public boolean inheritBuffers = true;
    public int vertexBuffers = 0;
    public int fragmentBuffers = 0;
    public int kernelBuffers = 0;

    public static final int SIZE = 28;

    int encodedSize() {
        return 28;
    }

    long encode(long p) {
        memPutInt(p, kinds);
        p += 4;
        memPutInt(p, count);
        p += 4;
        memPutInt(p, inheritPipeline ? 1 : 0);
        p += 4;
        memPutInt(p, inheritBuffers ? 1 : 0);
        p += 4;
        memPutInt(p, vertexBuffers);
        p += 4;
        memPutInt(p, fragmentBuffers);
        p += 4;
        memPutInt(p, kernelBuffers);
        p += 4;
        return p;
    }

    static IndirectInfo decode(long p) {
        var r = new IndirectInfo();
        r.kinds = memGetInt(p);
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        r.inheritPipeline = memGetInt(p) != 0;
        p += 4;
        r.inheritBuffers = memGetInt(p) != 0;
        p += 4;
        r.vertexBuffers = memGetInt(p);
        p += 4;
        r.fragmentBuffers = memGetInt(p);
        p += 4;
        r.kernelBuffers = memGetInt(p);
        p += 4;
        return r;
    }
}
