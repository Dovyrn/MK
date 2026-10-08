package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ImportedPassInfo {
    public int width = 1;
    public int height = 1;
    public int samples = 1;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, samples);
        p += 4;
        return p;
    }

    static ImportedPassInfo decode(long p) {
        var r = new ImportedPassInfo();
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.samples = memGetInt(p);
        p += 4;
        return r;
    }
}
