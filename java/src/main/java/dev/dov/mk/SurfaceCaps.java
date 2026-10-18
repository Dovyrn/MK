package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SurfaceCaps {
    public int minImages = 0;
    public int maxImages = 0;
    public int width = 0;
    public int height = 0;
    public boolean mailbox = false;
    public boolean immediate = false;
    public boolean premultiplied = false;

    public static final int SIZE = 28;

    int encodedSize() {
        return 28;
    }

    long encode(long p) {
        memPutInt(p, minImages);
        p += 4;
        memPutInt(p, maxImages);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, mailbox ? 1 : 0);
        p += 4;
        memPutInt(p, immediate ? 1 : 0);
        p += 4;
        memPutInt(p, premultiplied ? 1 : 0);
        p += 4;
        return p;
    }

    static SurfaceCaps decode(long p) {
        var r = new SurfaceCaps();
        r.minImages = memGetInt(p);
        p += 4;
        r.maxImages = memGetInt(p);
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.mailbox = memGetInt(p) != 0;
        p += 4;
        r.immediate = memGetInt(p) != 0;
        p += 4;
        r.premultiplied = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
