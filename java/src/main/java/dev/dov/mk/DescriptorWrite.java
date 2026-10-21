package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DescriptorWrite {
    public int binding = 0;
    public int element = 0;
    public Buffer buffer = null;
    public long bufferOffset = 0;
    public long bufferSize = -1;
    public Texture texture = null;
    public Sampler sampler = null;

    public static final int SIZE = 48;

    int encodedSize() {
        return 48;
    }

    long encode(long p) {
        memPutInt(p, binding);
        p += 4;
        memPutInt(p, element);
        p += 4;
        memPutLong(p, Buffer.raw(buffer));
        p += 8;
        memPutLong(p, bufferOffset);
        p += 8;
        memPutLong(p, bufferSize);
        p += 8;
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutLong(p, Sampler.raw(sampler));
        p += 8;
        return p;
    }

    static DescriptorWrite decode(long p) {
        var r = new DescriptorWrite();
        r.binding = memGetInt(p);
        p += 4;
        r.element = memGetInt(p);
        p += 4;
        r.buffer = Buffer.of(memGetLong(p));
        p += 8;
        r.bufferOffset = memGetLong(p);
        p += 8;
        r.bufferSize = memGetLong(p);
        p += 8;
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.sampler = Sampler.of(memGetLong(p));
        p += 8;
        return r;
    }
}
