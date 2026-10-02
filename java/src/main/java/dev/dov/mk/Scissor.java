package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class Scissor {
    public int x = 0;
    public int y = 0;
    public int width = 0;
    public int height = 0;

    public int size() {
        return 16;
    }

    public long write(long p) {
        memPutInt(p, x);
        p += 4;
        memPutInt(p, y);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        return p;
    }

    public static Scissor read(long p) {
        var r = new Scissor();
        r.x = memGetInt(p);
        p += 4;
        r.y = memGetInt(p);
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        return r;
    }
}
