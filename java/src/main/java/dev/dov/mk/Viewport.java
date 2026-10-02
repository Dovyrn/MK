package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class Viewport {
    public float x = 0.0f;
    public float y = 0.0f;
    public float width = 0.0f;
    public float height = 0.0f;
    public float near = 0.0f;
    public float far = 1.0f;

    public static final int SIZE = 24;

    int encodedSize() {
        return 24;
    }

    long encode(long p) {
        memPutFloat(p, x);
        p += 4;
        memPutFloat(p, y);
        p += 4;
        memPutFloat(p, width);
        p += 4;
        memPutFloat(p, height);
        p += 4;
        memPutFloat(p, near);
        p += 4;
        memPutFloat(p, far);
        p += 4;
        return p;
    }

    static Viewport decode(long p) {
        var r = new Viewport();
        r.x = memGetFloat(p);
        p += 4;
        r.y = memGetFloat(p);
        p += 4;
        r.width = memGetFloat(p);
        p += 4;
        r.height = memGetFloat(p);
        p += 4;
        r.near = memGetFloat(p);
        p += 4;
        r.far = memGetFloat(p);
        p += 4;
        return r;
    }
}
