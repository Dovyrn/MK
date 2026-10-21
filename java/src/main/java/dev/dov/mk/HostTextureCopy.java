package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class HostTextureCopy {
    public Texture texture = null;
    public int mip = 0;
    public int layer = 0;
    public int x = 0;
    public int y = 0;
    public int z = 0;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int rowTexels = 0;
    public int sliceRows = 0;

    public static final int SIZE = 48;

    int encodedSize() {
        return 48;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, mip);
        p += 4;
        memPutInt(p, layer);
        p += 4;
        memPutInt(p, x);
        p += 4;
        memPutInt(p, y);
        p += 4;
        memPutInt(p, z);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, rowTexels);
        p += 4;
        memPutInt(p, sliceRows);
        p += 4;
        return p;
    }

    static HostTextureCopy decode(long p) {
        var r = new HostTextureCopy();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.mip = memGetInt(p);
        p += 4;
        r.layer = memGetInt(p);
        p += 4;
        r.x = memGetInt(p);
        p += 4;
        r.y = memGetInt(p);
        p += 4;
        r.z = memGetInt(p);
        p += 4;
        r.width = memGetInt(p);
        p += 4;
        r.height = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        r.rowTexels = memGetInt(p);
        p += 4;
        r.sliceRows = memGetInt(p);
        p += 4;
        return r;
    }
}
