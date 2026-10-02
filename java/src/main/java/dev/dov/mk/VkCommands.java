package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VkCommands extends Commands {
    public void setLineWidth(float width) {
        var p = head(12, 4);
        memPutFloat(p, width);
    }

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(50, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void multiview(int views) {
        var p = head(54, 4);
        memPutInt(p, views);
    }

    public void executeGenerated(int layout, int indirect, long offset, long size, int preprocess, long preprocessOffset, long preprocessSize, int sequences, int count, long countOffset, int draws) {
        var p = head(59, 64);
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

    public void bindRayPipeline(int pipeline) {
        var p = head(60, 4);
        memPutInt(p, pipeline);
    }

    public void traceRays(int raygen, long raygenOffset, long raygenStride, long raygenSize, int miss, long missOffset, long missStride, long missSize, int hit, long hitOffset, long hitStride, long hitSize, int callable, long callableOffset, long callableStride, long callableSize, int width, int height, int depth) {
        var p = head(61, 124);
        memPutInt(p, raygen);
        memPutLong(p + 4, raygenOffset);
        memPutLong(p + 12, raygenStride);
        memPutLong(p + 20, raygenSize);
        memPutInt(p + 28, miss);
        memPutLong(p + 32, missOffset);
        memPutLong(p + 40, missStride);
        memPutLong(p + 48, missSize);
        memPutInt(p + 56, hit);
        memPutLong(p + 60, hitOffset);
        memPutLong(p + 68, hitStride);
        memPutLong(p + 76, hitSize);
        memPutInt(p + 84, callable);
        memPutLong(p + 88, callableOffset);
        memPutLong(p + 96, callableStride);
        memPutLong(p + 104, callableSize);
        memPutInt(p + 112, width);
        memPutInt(p + 116, height);
        memPutInt(p + 120, depth);
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

    public void beginConditional(int buffer, long offset, int inverted) {
        var p = head(70, 16);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, inverted);
    }

    public void endConditional() {
        head(71, 0);
    }
}
