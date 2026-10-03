package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class Budget {
    public long budget = 0;
    public long usage = 0;

    public static final int SIZE = 16;

    int encodedSize() {
        return 16;
    }

    long encode(long p) {
        memPutLong(p, budget);
        p += 8;
        memPutLong(p, usage);
        p += 8;
        return p;
    }

    static Budget decode(long p) {
        var r = new Budget();
        r.budget = memGetLong(p);
        p += 8;
        r.usage = memGetLong(p);
        p += 8;
        return r;
    }
}
