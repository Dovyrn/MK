package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class GlCommands extends Commands {

    public void drawIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(42, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void drawIndexedIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(43, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }
}
