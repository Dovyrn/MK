package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class TextureBarrier {
    public int texture = 0;
    public int srcStage = 0;
    public int srcAccess = 0;
    public int dstStage = 0;
    public int dstAccess = 0;
    public int before = Layout.UNDEFINED;
    public int after = Layout.UNDEFINED;
    public int baseMip = 0;
    public int mips = -1;
    public int baseLayer = 0;
    public int layers = -1;

    public int size() {
        return 44;
    }

    public long write(long p) {
        memPutInt(p, texture);
        p += 4;
        memPutInt(p, srcStage);
        p += 4;
        memPutInt(p, srcAccess);
        p += 4;
        memPutInt(p, dstStage);
        p += 4;
        memPutInt(p, dstAccess);
        p += 4;
        memPutInt(p, before);
        p += 4;
        memPutInt(p, after);
        p += 4;
        memPutInt(p, baseMip);
        p += 4;
        memPutInt(p, mips);
        p += 4;
        memPutInt(p, baseLayer);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        return p;
    }

    public static TextureBarrier read(long p) {
        var r = new TextureBarrier();
        r.texture = memGetInt(p);
        p += 4;
        r.srcStage = memGetInt(p);
        p += 4;
        r.srcAccess = memGetInt(p);
        p += 4;
        r.dstStage = memGetInt(p);
        p += 4;
        r.dstAccess = memGetInt(p);
        p += 4;
        r.before = memGetInt(p);
        p += 4;
        r.after = memGetInt(p);
        p += 4;
        r.baseMip = memGetInt(p);
        p += 4;
        r.mips = memGetInt(p);
        p += 4;
        r.baseLayer = memGetInt(p);
        p += 4;
        r.layers = memGetInt(p);
        p += 4;
        return r;
    }
}
