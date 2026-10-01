package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VkCommands extends Commands {
    public void setLineWidth(float width) {
        var p = head(12, 4);
        memPutFloat(p, width);
    }

    public void executeGenerated(int layout, int indirect, long offset, long size, int preprocess, long preprocessOffset, long preprocessSize, int sequences, int count, long countOffset, int draws) {
        var p = head(54, 64);
        memPutInt(p, layout);
        memPutInt(p + 4, indirect);
        memPutLong(p + 8, offset);
        memPutLong(p + 16, size);
        memPutInt(p + 24, preprocess);
        memPutLong(p + 28, preprocessOffset);
        memPutLong(p + 36, preprocessSize);
        memPutInt(p + 44, sequences);
        memPutInt(p + 48, count);
        memPutLong(p + 52, countOffset);
        memPutInt(p + 60, draws);
    }
}
