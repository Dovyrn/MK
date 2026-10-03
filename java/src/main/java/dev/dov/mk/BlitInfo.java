package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class BlitInfo {
    public int src = 0;
    public int srcLayout = Layout.COPY_SRC;
    public int srcMip = 0;
    public int srcLayer = 0;
    public int srcX = 0;
    public int srcY = 0;
    public int srcWidth = 1;
    public int srcHeight = 1;
    public int dst = 0;
    public int dstLayout = Layout.COPY_DST;
    public int dstMip = 0;
    public int dstLayer = 0;
    public int dstX = 0;
    public int dstY = 0;
    public int dstWidth = 1;
    public int dstHeight = 1;
    public int filter = Filter.LINEAR;

    public static final int SIZE = 68;

    int encodedSize() {
        return 68;
    }

    long encode(long p) {
        memPutInt(p, src);
        p += 4;
        memPutInt(p, srcLayout);
        p += 4;
        memPutInt(p, srcMip);
        p += 4;
        memPutInt(p, srcLayer);
        p += 4;
        memPutInt(p, srcX);
        p += 4;
        memPutInt(p, srcY);
        p += 4;
        memPutInt(p, srcWidth);
        p += 4;
        memPutInt(p, srcHeight);
        p += 4;
        memPutInt(p, dst);
        p += 4;
        memPutInt(p, dstLayout);
        p += 4;
        memPutInt(p, dstMip);
        p += 4;
        memPutInt(p, dstLayer);
        p += 4;
        memPutInt(p, dstX);
        p += 4;
        memPutInt(p, dstY);
        p += 4;
        memPutInt(p, dstWidth);
        p += 4;
        memPutInt(p, dstHeight);
        p += 4;
        memPutInt(p, filter);
        p += 4;
        return p;
    }

    static BlitInfo decode(long p) {
        var r = new BlitInfo();
        r.src = memGetInt(p);
        p += 4;
        r.srcLayout = memGetInt(p);
        p += 4;
        r.srcMip = memGetInt(p);
        p += 4;
        r.srcLayer = memGetInt(p);
        p += 4;
        r.srcX = memGetInt(p);
        p += 4;
        r.srcY = memGetInt(p);
        p += 4;
        r.srcWidth = memGetInt(p);
        p += 4;
        r.srcHeight = memGetInt(p);
        p += 4;
        r.dst = memGetInt(p);
        p += 4;
        r.dstLayout = memGetInt(p);
        p += 4;
        r.dstMip = memGetInt(p);
        p += 4;
        r.dstLayer = memGetInt(p);
        p += 4;
        r.dstX = memGetInt(p);
        p += 4;
        r.dstY = memGetInt(p);
        p += 4;
        r.dstWidth = memGetInt(p);
        p += 4;
        r.dstHeight = memGetInt(p);
        p += 4;
        r.filter = memGetInt(p);
        p += 4;
        return r;
    }
}
