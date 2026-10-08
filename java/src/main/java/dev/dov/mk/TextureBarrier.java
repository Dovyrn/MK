package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureBarrier {
    public Texture texture = null;
    public int srcStage = 0;
    public int dstStage = 0;
    public TextureLayout before = TextureLayout.GENERAL;
    public TextureLayout after = TextureLayout.GENERAL;
    public int baseMip = 0;
    public int mips = -1;
    public int baseLayer = 0;
    public int layers = -1;

    public static final int SIZE = 40;

    int encodedSize() {
        return 40;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, srcStage);
        p += 4;
        memPutInt(p, dstStage);
        p += 4;
        memPutInt(p, before.value);
        p += 4;
        memPutInt(p, after.value);
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

    static TextureBarrier decode(long p) {
        var r = new TextureBarrier();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.srcStage = memGetInt(p);
        p += 4;
        r.dstStage = memGetInt(p);
        p += 4;
        r.before = TextureLayout.of(memGetInt(p));
        p += 4;
        r.after = TextureLayout.of(memGetInt(p));
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
