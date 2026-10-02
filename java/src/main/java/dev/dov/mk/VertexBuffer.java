package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class VertexBuffer {
    public int stride = 0;
    public boolean instance = false;

    public static final int SIZE = 8;

    int encodedSize() {
        return 8;
    }

    long encode(long p) {
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, instance ? 1 : 0);
        p += 4;
        return p;
    }

    static VertexBuffer decode(long p) {
        var r = new VertexBuffer();
        r.stride = memGetInt(p);
        p += 4;
        r.instance = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
