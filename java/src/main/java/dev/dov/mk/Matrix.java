package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class Matrix {
    public int m = 0;
    public int n = 0;
    public int k = 0;
    public int a = Element.F16;
    public int b = Element.F16;
    public int c = Element.F32;
    public int result = Element.F32;

    public static final int SIZE = 28;

    int encodedSize() {
        return 28;
    }

    long encode(long p) {
        memPutInt(p, m);
        p += 4;
        memPutInt(p, n);
        p += 4;
        memPutInt(p, k);
        p += 4;
        memPutInt(p, a);
        p += 4;
        memPutInt(p, b);
        p += 4;
        memPutInt(p, c);
        p += 4;
        memPutInt(p, result);
        p += 4;
        return p;
    }

    static Matrix decode(long p) {
        var r = new Matrix();
        r.m = memGetInt(p);
        p += 4;
        r.n = memGetInt(p);
        p += 4;
        r.k = memGetInt(p);
        p += 4;
        r.a = memGetInt(p);
        p += 4;
        r.b = memGetInt(p);
        p += 4;
        r.c = memGetInt(p);
        p += 4;
        r.result = memGetInt(p);
        p += 4;
        return r;
    }
}
