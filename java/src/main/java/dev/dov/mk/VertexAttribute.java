package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class VertexAttribute {
    public int location = 0;
    public int buffer = 0;
    public int format = 0;
    public int offset = 0;

    public static final int SIZE = 16;

    int encodedSize() {
        return 16;
    }

    long encode(long p) {
        memPutInt(p, location);
        p += 4;
        memPutInt(p, buffer);
        p += 4;
        memPutInt(p, format);
        p += 4;
        memPutInt(p, offset);
        p += 4;
        return p;
    }

    static VertexAttribute decode(long p) {
        var r = new VertexAttribute();
        r.location = memGetInt(p);
        p += 4;
        r.buffer = memGetInt(p);
        p += 4;
        r.format = memGetInt(p);
        p += 4;
        r.offset = memGetInt(p);
        p += 4;
        return r;
    }
}
