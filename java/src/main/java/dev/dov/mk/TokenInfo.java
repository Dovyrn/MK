package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TokenInfo {
    public int kind = Token.DRAW;
    public int offset = 0;
    public int pushOffset = 0;
    public int pushSize = 0;
    public int binding = 0;

    public static final int SIZE = 20;

    int encodedSize() {
        return 20;
    }

    long encode(long p) {
        memPutInt(p, kind);
        p += 4;
        memPutInt(p, offset);
        p += 4;
        memPutInt(p, pushOffset);
        p += 4;
        memPutInt(p, pushSize);
        p += 4;
        memPutInt(p, binding);
        p += 4;
        return p;
    }

    static TokenInfo decode(long p) {
        var r = new TokenInfo();
        r.kind = memGetInt(p);
        p += 4;
        r.offset = memGetInt(p);
        p += 4;
        r.pushOffset = memGetInt(p);
        p += 4;
        r.pushSize = memGetInt(p);
        p += 4;
        r.binding = memGetInt(p);
        p += 4;
        return r;
    }
}
