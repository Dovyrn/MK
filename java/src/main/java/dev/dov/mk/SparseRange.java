package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SparseRange {
    public int page = 0;
    public int count = 1;
    public int heapPage = 0;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, page);
        p += 4;
        memPutInt(p, count);
        p += 4;
        memPutInt(p, heapPage);
        p += 4;
        return p;
    }

    static SparseRange decode(long p) {
        var r = new SparseRange();
        r.page = memGetInt(p);
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        r.heapPage = memGetInt(p);
        p += 4;
        return r;
    }
}
