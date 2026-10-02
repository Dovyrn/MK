package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class GeneratedInfo {
    public int layout = 0;
    public int indirect = 0;
    public long offset = 0;
    public long size = 0;
    public int preprocess = -1;
    public long preprocessOffset = 0;
    public long preprocessSize = 0;
    public int sequences = 0;
    public int count = -1;
    public long countOffset = 0;
    public int draws = 0;

    public static final int SIZE = 64;

    int encodedSize() {
        return 64;
    }

    long encode(long p) {
        memPutInt(p, layout);
        p += 4;
        memPutInt(p, indirect);
        p += 4;
        memPutLong(p, offset);
        p += 8;
        memPutLong(p, size);
        p += 8;
        memPutInt(p, preprocess);
        p += 4;
        memPutLong(p, preprocessOffset);
        p += 8;
        memPutLong(p, preprocessSize);
        p += 8;
        memPutInt(p, sequences);
        p += 4;
        memPutInt(p, count);
        p += 4;
        memPutLong(p, countOffset);
        p += 8;
        memPutInt(p, draws);
        p += 4;
        return p;
    }

    static GeneratedInfo decode(long p) {
        var r = new GeneratedInfo();
        r.layout = memGetInt(p);
        p += 4;
        r.indirect = memGetInt(p);
        p += 4;
        r.offset = memGetLong(p);
        p += 8;
        r.size = memGetLong(p);
        p += 8;
        r.preprocess = memGetInt(p);
        p += 4;
        r.preprocessOffset = memGetLong(p);
        p += 8;
        r.preprocessSize = memGetLong(p);
        p += 8;
        r.sequences = memGetInt(p);
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        r.countOffset = memGetLong(p);
        p += 8;
        r.draws = memGetInt(p);
        p += 4;
        return r;
    }
}
