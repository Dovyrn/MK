package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VkCommands extends Commands {

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(55, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void setShadingRate(int width, int height, int primitive, int attachment) {
        var p = head(63, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, primitive);
        memPutInt(p + 12, attachment);
    }

    public void rateAttachment(int texture, int layout, int texelWidth, int texelHeight) {
        var p = head(64, 16);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, texelWidth);
        memPutInt(p + 12, texelHeight);
    }

    public void writeCompactedSize(int accel, int heap, int index) {
        var p = head(65, 12);
        memPutInt(p, accel);
        memPutInt(p + 4, heap);
        memPutInt(p + 8, index);
    }

    public void copyAccel(int src, int dst, int compact) {
        var p = head(66, 12);
        memPutInt(p, src);
        memPutInt(p + 4, dst);
        memPutInt(p + 8, compact);
    }
}
