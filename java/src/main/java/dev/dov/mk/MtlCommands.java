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
        var p = head(52, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void tilePass(int width, int height, int imageblock, int memory) {
        var p = head(53, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, imageblock);
        memPutInt(p + 12, memory);
    }

    public void dispatchTile(int width, int height) {
        var p = head(54, 8);
        memPutInt(p, width);
        memPutInt(p + 4, height);
    }

    public void tileMemory(int length, int offset, int index) {
        var p = head(55, 12);
        memPutInt(p, length);
        memPutInt(p + 4, offset);
        memPutInt(p + 8, index);
    }

    public void multiview(int views) {
        var p = head(56, 4);
        memPutInt(p, views);
    }

    public void executeIndirect(int indirect, int start, int count) {
        var p = head(58, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void resetIndirect(int indirect, int start, int count) {
        var p = head(59, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void optimizeIndirect(int indirect, int start, int count) {
        var p = head(60, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void rateMap(int map) {
        var p = head(64, 4);
        memPutInt(p, map);
    }

    public void writeCompactedSize(int accel, int heap, int index) {
        var p = head(67, 12);
        memPutInt(p, accel);
        memPutInt(p + 4, heap);
        memPutInt(p + 8, index);
    }

    public void copyAccel(int src, int dst, int compact) {
        var p = head(68, 12);
        memPutInt(p, src);
        memPutInt(p + 4, dst);
        memPutInt(p + 8, compact);
    }

    public void bindTensor(int slot, int tensor) {
        var p = head(74, 8);
        memPutInt(p, slot);
        memPutInt(p + 4, tensor);
    }

    public void dispatchNetwork(int pipeline, int heap) {
        var p = head(75, 8);
        memPutInt(p, pipeline);
        memPutInt(p + 4, heap);
    }
}
