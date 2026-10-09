package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class ColorAttachment {
    public Texture texture = null;
    public LoadOp load = LoadOp.CLEAR;
    public StoreOp store = StoreOp.STORE;
    public float[] clear = new float[] {0.0f, 0.0f, 0.0f, 0.0f};
    public Texture resolve = null;

    public static final int SIZE = 40;

    int encodedSize() {
        return 40;
    }

    long encode(long p) {
        memPutLong(p, Texture.raw(texture));
        p += 8;
        memPutInt(p, load.value);
        p += 4;
        memPutInt(p, store.value);
        p += 4;
        if (clear.length != 4) {
            throw new IllegalArgumentException("clear has 4 values");
        }
        for (var x : clear) {
            memPutFloat(p, x);
            p += 4;
        }
        memPutLong(p, Texture.raw(resolve));
        p += 8;
        return p;
    }

    static ColorAttachment decode(long p) {
        var r = new ColorAttachment();
        r.texture = Texture.of(memGetLong(p));
        p += 8;
        r.load = LoadOp.of(memGetInt(p));
        p += 4;
        r.store = StoreOp.of(memGetInt(p));
        p += 4;
        for (var i = 0; i < 4; i++) {
            r.clear[i] = memGetFloat(p);
            p += 4;
        }
        r.resolve = Texture.of(memGetLong(p));
        p += 8;
        return r;
    }
}
