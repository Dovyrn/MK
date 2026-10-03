package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class IndirectInfo {
    public int size = 0;
    public int pipeline = 0;
    public boolean indexed = false;
    public int indices = 0;
    public long offset = 0;
    public int kind = Index.U16;

    public static final int SIZE = 28;

    int encodedSize() {
        return 28;
    }

    long encode(long p) {
        memPutInt(p, size);
        p += 4;
        memPutInt(p, pipeline);
        p += 4;
        memPutInt(p, indexed ? 1 : 0);
        p += 4;
        memPutInt(p, indices);
        p += 4;
        memPutLong(p, offset);
        p += 8;
        memPutInt(p, kind);
        p += 4;
        return p;
    }

    static IndirectInfo decode(long p) {
        var r = new IndirectInfo();
        r.size = memGetInt(p);
        p += 4;
        r.pipeline = memGetInt(p);
        p += 4;
        r.indexed = memGetInt(p) != 0;
        p += 4;
        r.indices = memGetInt(p);
        p += 4;
        r.offset = memGetLong(p);
        p += 8;
        r.kind = memGetInt(p);
        p += 4;
        return r;
    }
}
