package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class Scissor {
    public int x = 0;
    public int y = 0;
    public int width = 0;
    public int height = 0;

    public static final int SIZE = 16;

    int encodedSize() {
        return 16;
    }

    long encode(long p) {
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

    static Scissor decode(long p) {
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
