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
        var p = head(51, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, imageblock);
        memPutInt(p + 12, memory);
    }

    public void dispatchTile(int width, int height) {
        var p = head(52, 8);
        memPutInt(p, width);
        memPutInt(p + 4, height);
    }

    public void tileMemory(int length, int offset, int index) {
        var p = head(53, 12);
        memPutInt(p, length);
        memPutInt(p + 4, offset);
        memPutInt(p + 8, index);
    }

    public void executeIndirect(int indirect, int start, int count) {
        var p = head(56, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void resetIndirect(int indirect, int start, int count) {
        var p = head(57, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void optimizeIndirect(int indirect, int start, int count) {
        var p = head(58, 12);
        memPutInt(p, indirect);
        memPutInt(p + 4, start);
        memPutInt(p + 8, count);
    }

    public void rateMap(int map) {
        var p = head(62, 4);
        memPutInt(p, map);
    }

    public void upscale(int upscaler, int color, int output, int depth, int motion, float jitterX, float jitterY, int reset) {
        var p = head(67, 32);
        memPutInt(p, upscaler);
        memPutInt(p + 4, color);
        memPutInt(p + 8, output);
        memPutInt(p + 12, depth);
        memPutInt(p + 16, motion);
        memPutFloat(p + 20, jitterX);
        memPutFloat(p + 24, jitterY);
        memPutInt(p + 28, reset);
    }
}
