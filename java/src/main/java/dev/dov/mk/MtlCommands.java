package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class MtlCommands extends Commands {

    public void dispatchThreads(int x, int y, int z) {
        var p = head(11, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(55, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void executeIndirect(int indirect, int start, int count) {
        var p = head(57, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void resetIndirect(int indirect, int start, int count) {
        var p = head(58, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void optimizeIndirect(int indirect, int start, int count) {
        var p = head(59, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void writeCompactedSize(int accel, int heap, int index) {
        var p = head(62, 12);
        memPutInt(p, accel);
        memPutInt(p + 4, heap);
        memPutInt(p + 8, index);
    }

    public void copyAccel(int src, int dst, int compact) {
        var p = head(63, 12);
        memPutInt(p, src);
        memPutInt(p + 4, dst);
        memPutInt(p + 8, compact);
    }
}
