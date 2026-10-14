package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureDiscard {
    public Texture texture = null;
    public int srcStage = 0;
    public int dstStage = 0;

    public static final int SIZE = 16;

    int encodedSize() {
        return 16;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, srcStage);
        p += 4;
        memPutInt(p, dstStage);
        p += 4;
        return p;
    }

    static TextureDiscard decode(long p) {
        var r = new TextureDiscard();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.srcStage = memGetInt(p);
        p += 4;
        r.dstStage = memGetInt(p);
        p += 4;
        return r;
    }
}
