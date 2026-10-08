package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TileMemory {
    public int width = 0;
    public int height = 0;
    public int shared = 0;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, shared);
        p += 4;
        return p;
    }

    static TileMemory decode(long p) {
        var r = new TileMemory();
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.shared = memGetInt(p);
        p += 4;
        return r;
    }
}
