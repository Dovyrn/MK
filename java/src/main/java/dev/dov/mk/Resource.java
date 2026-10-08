package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class Resource {
    public ResourceKind kind = ResourceKind.BUFFER;
    public long handle = 0;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, kind.value);
        p += 4;
        memPutLong(p, handle);
        p += 8;
        return p;
    }

    static Resource decode(long p) {
        var r = new Resource();
        r.kind = ResourceKind.of(memGetInt(p));
        p += 4;
        r.handle = memGetLong(p);
        p += 8;
        return r;
    }
}
