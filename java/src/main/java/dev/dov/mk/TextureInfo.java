package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureInfo {
    public TextureDimension dimension = TextureDimension.D2;
    public Format format = Format.NONE;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int layers = 1;
    public int mips = 1;
    public int samples = 1;
    public int usage = 0;
    public Memory memory = Memory.DEVICE;

    public static final int SIZE = 40;

    int encodedSize() {
        return 40;
    }

    long encode(long p) {
        memPutInt(p, dimension.value);
        p += 4;
        memPutInt(p, format.value);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        memPutInt(p, mips);
        p += 4;
        memPutInt(p, samples);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        memPutInt(p, memory.value);
        p += 4;
        return p;
    }

    static TextureInfo decode(long p) {
        var r = new TextureInfo();
        r.dimension = TextureDimension.of(memGetInt(p));
        p += 4;
        r.format = Format.of(memGetInt(p));
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        r.layers = memGetInt(p);
        p += 4;
        r.mips = memGetInt(p);
        p += 4;
        r.samples = memGetInt(p);
        p += 4;
        r.usage = memGetInt(p);
        p += 4;
        r.memory = Memory.of(memGetInt(p));
        p += 4;
        return r;
    }
}
