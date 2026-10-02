package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class LayoutInfo {
    public int stride = 0;
    public boolean compute = false;
    public TokenInfo[] tokens = new TokenInfo[0];

    int encodedSize() {
        var tokensSize = 4;
        for (var x : tokens) {
            tokensSize += x.encodedSize();
        }
        return 8 + tokensSize;
    }

    long encode(long p) {
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, compute ? 1 : 0);
        p += 4;
        memPutInt(p, tokens.length);
        p += 4;
        for (var x : tokens) {
            p = x.encode(p);
        }
        return p;
    }
}
