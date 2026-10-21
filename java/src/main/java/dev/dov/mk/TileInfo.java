package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TileInfo {
    public int width = 0;
    public int height = 0;
    public int threadgroupMemory = 0;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, threadgroupMemory);
        p += 4;
        return p;
    }

    static TileInfo decode(long p) {
        var r = new TileInfo();
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.threadgroupMemory = memGetInt(p);
        p += 4;
        return r;
    }
}
