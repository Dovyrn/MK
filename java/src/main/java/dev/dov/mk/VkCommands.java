package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VkCommands extends Commands {

    public void setLineWidth(float width) {
        var p = head(12, 4);
        memPutFloat(p, width);
    }

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(52, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void multiview(int views) {
        var p = head(56, 4);
        memPutInt(p, views);
    }

    public void executeGenerated(GeneratedInfo info) {
        var p = head(61, 64);
        memPutInt(p, info.layout);
        memPutInt(p + 4, info.indirect);
        memPutLong(p + 8, info.offset);
        memPutLong(p + 16, info.size);
        memPutInt(p + 24, info.preprocess);
        memPutLong(p + 28, info.preprocessOffset);
        memPutLong(p + 36, info.preprocessSize);
        memPutInt(p + 44, info.sequences);
        memPutInt(p + 48, info.count);
        memPutLong(p + 52, info.countOffset);
        memPutInt(p + 60, info.draws);
    }

    public void bindRayPipeline(int pipeline) {
        var p = head(62, 4);
        memPutInt(p, pipeline);
    }

    public void traceRays(TraceInfo info) {
        var p = head(63, 124);
        memPutInt(p, info.raygen);
        memPutLong(p + 4, info.raygenOffset);
        memPutLong(p + 12, info.raygenStride);
        memPutLong(p + 20, info.raygenSize);
        memPutInt(p + 28, info.miss);
        memPutLong(p + 32, info.missOffset);
        memPutLong(p + 40, info.missStride);
        memPutLong(p + 48, info.missSize);
        memPutInt(p + 56, info.hit);
        memPutLong(p + 60, info.hitOffset);
        memPutLong(p + 68, info.hitStride);
        memPutLong(p + 76, info.hitSize);
        memPutInt(p + 84, info.callable);
        memPutLong(p + 88, info.callableOffset);
        memPutLong(p + 96, info.callableStride);
        memPutLong(p + 104, info.callableSize);
        memPutInt(p + 112, info.width);
        memPutInt(p + 116, info.height);
        memPutInt(p + 120, info.depth);
    }

    public void setShadingRate(int width, int height, int primitive, int attachment) {
        var p = head(65, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, primitive);
        memPutInt(p + 12, attachment);
    }

    public void rateAttachment(int texture, int layout, int texelWidth, int texelHeight) {
        var p = head(66, 16);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, texelWidth);
        memPutInt(p + 12, texelHeight);
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

    public void beginConditional(int buffer, long offset, int inverted) {
        var p = head(72, 16);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, inverted);
    }

    public void endConditional() {
        head(73, 0);
    }
}
