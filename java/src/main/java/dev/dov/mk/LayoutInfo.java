package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class LayoutInfo {
    public int stride = 0;
    public boolean compute = false;
    public TokenInfo[] tokens = new TokenInfo[0];

    public int size() {
        var tokensSize = 0;
        for (var x : tokens) {
            tokensSize += x.size();
        }
        return 12 + tokensSize;
    }

    public long write(long p) {
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, compute ? 1 : 0);
        p += 4;
        memPutInt(p, tokens.length);
        p += 4;
        for (var x : tokens) {
            p = x.write(p);
        }
        return p;
    }
}
