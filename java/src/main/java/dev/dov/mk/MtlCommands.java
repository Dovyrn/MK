package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class MtlCommands extends Commands {

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(54, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void writeCompactedSize(int accel, int heap, int index) {
        var p = head(58, 12);
        memPutInt(p, accel);
        memPutInt(p + 4, heap);
        memPutInt(p + 8, index);
    }

    public void copyAccel(int src, int dst, int compact) {
        var p = head(59, 12);
        memPutInt(p, src);
        memPutInt(p + 4, dst);
        memPutInt(p + 8, compact);
    }
}
