package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureViewInfo {
    public TextureDimension dimension = TextureDimension.D2;
    public Format format = Format.NONE;
    public int baseMip = 0;
    public int mips = 1;
    public int baseLayer = 0;
    public int layers = 1;
    public int usage = TextureUsage.SAMPLED;

    public static final int SIZE = 28;

    int encodedSize() {
        return 28;
    }

    long encode(long p) {
        memPutInt(p, dimension.value);
        p += 4;
        memPutInt(p, format.value);
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

    static TextureViewInfo decode(long p) {
        var r = new TextureViewInfo();
        r.dimension = TextureDimension.of(memGetInt(p));
        p += 4;
        r.format = Format.of(memGetInt(p));
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
