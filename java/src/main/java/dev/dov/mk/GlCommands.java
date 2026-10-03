package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class GlCommands extends Commands {

    public void drawIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(45, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void drawIndexedIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(46, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void drawMeshIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(49, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void setShadingRate(int width, int height, int primitive, int attachment) {
        var p = head(60, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, primitive);
        memPutInt(p + 12, attachment);
    }

    public void rateAttachment(int texture, int layout, int texelWidth, int texelHeight) {
        var p = head(61, 16);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, texelWidth);
        memPutInt(p + 12, texelHeight);
    }
}
