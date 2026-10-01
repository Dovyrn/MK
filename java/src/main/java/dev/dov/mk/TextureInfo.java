package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class TextureInfo {
    public int dimension = Dimension.D2;
    public int format = Format.NONE;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int layers = 1;
    public int mips = 1;
    public int samples = 1;
    public int usage = 0;
    public int memory = Memory.DEVICE;

    public int size() {
        return 40;
    }

    public long write(long p) {
        memPutInt(p, dimension);
        p += 4;
        memPutInt(p, format);
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
        memPutInt(p, memory);
        p += 4;
        return p;
    }

    public static TextureInfo read(long p) {
        var r = new TextureInfo();
        r.dimension = memGetInt(p);
        p += 4;
        r.format = memGetInt(p);
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
        r.memory = memGetInt(p);
        p += 4;
        return r;
    }
}
