package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class MtlCommands extends Commands {
    public void dispatchThreads(int x, int y, int z) {
        var p = head(11, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }

    public void tilePass(int width, int height, int imageblock, int memory) {
        var p = head(46, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, imageblock);
        memPutInt(p + 12, memory);
    }

    public void dispatchTile(int width, int height) {
        var p = head(47, 8);
        memPutInt(p, width);
        memPutInt(p + 4, height);
    }

    public void tileMemory(int length, int offset, int index) {
        var p = head(48, 12);
        memPutInt(p, length);
        memPutInt(p + 4, offset);
        memPutInt(p + 8, index);
    }

    public void executeIndirect(int indirect, int start, int count) {
        var p = head(51, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void resetIndirect(int indirect, int start, int count) {
        var p = head(52, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void optimizeIndirect(int indirect, int start, int count) {
        var p = head(53, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }
}
