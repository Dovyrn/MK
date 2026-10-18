package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DescriptorBinding {
    public int binding = 0;
    public DescriptorType kind = DescriptorType.UNIFORM_BUFFER;
    public int count = 1;

    public static final int SIZE = 12;

    int encodedSize() {
        return 12;
    }

    long encode(long p) {
        memPutInt(p, binding);
        p += 4;
        memPutInt(p, kind.value);
        p += 4;
        memPutInt(p, count);
        p += 4;
        return p;
    }

    static DescriptorBinding decode(long p) {
        var r = new DescriptorBinding();
        r.binding = memGetInt(p);
        p += 4;
        r.kind = DescriptorType.of(memGetInt(p));
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        return r;
    }
}
