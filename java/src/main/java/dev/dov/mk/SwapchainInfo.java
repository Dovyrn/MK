package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SwapchainInfo {
    public Format format = Format.BGRA8_UNORM;
    public int width = 1;
    public int height = 1;
    public int images = 3;
    public int usage = TextureUsage.COLOR;
    public PresentMode mode = PresentMode.FIFO;
    public ColorSpace colorSpace = ColorSpace.SRGB;
    public boolean premultiplied = false;

    public static final int SIZE = 32;

    int encodedSize() {
        return 32;
    }

    long encode(long p) {
        memPutInt(p, format.value);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, images);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        memPutInt(p, mode.value);
        p += 4;
        memPutInt(p, colorSpace.value);
        p += 4;
        memPutInt(p, premultiplied ? 1 : 0);
        p += 4;
        return p;
    }

    static SwapchainInfo decode(long p) {
        var r = new SwapchainInfo();
        r.format = Format.of(memGetInt(p));
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.images = memGetInt(p);
        p += 4;
        r.usage = memGetInt(p);
        p += 4;
        r.mode = PresentMode.of(memGetInt(p));
        p += 4;
        r.colorSpace = ColorSpace.of(memGetInt(p));
        p += 4;
        r.premultiplied = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
