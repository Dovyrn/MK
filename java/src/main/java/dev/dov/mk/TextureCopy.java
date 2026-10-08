package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureCopy {
    public Texture src = null;
    public int srcMip = 0;
    public int srcLayer = 0;
    public int srcX = 0;
    public int srcY = 0;
    public int srcZ = 0;
    public Texture dst = null;
    public int dstMip = 0;
    public int dstLayer = 0;
    public int dstX = 0;
    public int dstY = 0;
    public int dstZ = 0;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int layers = 1;

    public static final int SIZE = 72;

    int encodedSize() {
        return 72;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(src));
        p += 8;
        memPutInt(p, srcMip);
        p += 4;
        memPutInt(p, srcLayer);
        p += 4;
        memPutInt(p, srcX);
        p += 4;
        memPutInt(p, srcY);
        p += 4;
        memPutInt(p, srcZ);
        p += 4;
        memPutLong(p, Texture.raw(dst));
        p += 8;
        memPutInt(p, dstMip);
        p += 4;
        memPutInt(p, dstLayer);
        p += 4;
        memPutInt(p, dstX);
        p += 4;
        memPutInt(p, dstY);
        p += 4;
        memPutInt(p, dstZ);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        return p;
    }

    static TextureCopy decode(long p) {
        var r = new TextureCopy();
        r.src = Texture.of(memGetLong(p));
        p += 8;
        r.srcMip = memGetInt(p);
        p += 4;
        r.srcLayer = memGetInt(p);
        p += 4;
        r.srcX = memGetInt(p);
        p += 4;
        r.srcY = memGetInt(p);
        p += 4;
        r.srcZ = memGetInt(p);
        p += 4;
        r.dst = Texture.of(memGetLong(p));
        p += 8;
        r.dstMip = memGetInt(p);
        p += 4;
        r.dstLayer = memGetInt(p);
        p += 4;
        r.dstX = memGetInt(p);
        p += 4;
        r.dstY = memGetInt(p);
        p += 4;
        r.dstZ = memGetInt(p);
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        r.layers = memGetInt(p);
        p += 4;
        return r;
    }
}
