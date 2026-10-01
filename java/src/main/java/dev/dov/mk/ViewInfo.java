package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class ViewInfo {
    public int dimension = Dimension.D2;
    public int format = Format.NONE;
    public int baseMip = 0;
    public int mips = 1;
    public int baseLayer = 0;
    public int layers = 1;
    public int usage = Usage.SAMPLED;

    public int size() {
        return 28;
    }

    public long write(long p) {
        memPutInt(p, dimension);
        p += 4;
        memPutInt(p, format);
        p += 4;
        memPutInt(p, baseMip);
        p += 4;
        memPutInt(p, mips);
        p += 4;
        memPutInt(p, baseLayer);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        return p;
    }

    public static ViewInfo read(long p) {
        var r = new ViewInfo();
        r.dimension = memGetInt(p);
        p += 4;
        r.format = memGetInt(p);
        p += 4;
        r.baseMip = memGetInt(p);
        p += 4;
        r.mips = memGetInt(p);
        p += 4;
        r.baseLayer = memGetInt(p);
        p += 4;
        r.layers = memGetInt(p);
        p += 4;
        r.usage = memGetInt(p);
        p += 4;
        return r;
    }
}
