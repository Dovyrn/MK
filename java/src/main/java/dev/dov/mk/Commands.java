package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Getter;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

@Getter
public class Commands {
    private long address = nmemAlloc(4096);
    private int capacity = 4096;
    private int size;

    public void reset() {
        size = 0;
    }

    public void free() {
        nmemFree(address);
    }

    protected long head(int op, int length) {
        if (size + 4 + length > capacity) {
            capacity = Math.max(capacity * 2, size + 4 + length);
            address = nmemRealloc(address, capacity);
        }
        var p = address + size;
        memPutShort(p, (short) op);
        memPutShort(p + 2, (short) length);
        size += 4 + length;
        return p + 4;
    }

    public void beginCompute() {
        head(0, 0);
    }

    public void endCompute() {
        head(1, 0);
    }

    public void bindPipeline(int pipeline) {
        var p = head(2, 4);
        memPutInt(p, pipeline);
    }

    public void pushConstants(int offset, ByteBuffer data) {
        var p = head(3, 4 + data.remaining());
        memPutInt(p, offset);
        memCopy(memAddress(data), p + 4, data.remaining());
    }

    public void dispatch(int x, int y, int z) {
        var p = head(4, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }

    public void dispatchIndirect(int buffer, long offset) {
        var p = head(5, 12);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
    }

    public void copyBuffer(int src, long srcOffset, int dst, long dstOffset, long size) {
        var p = head(6, 32);
        memPutInt(p, src);
        memPutLong(p + 4, srcOffset);
        memPutInt(p + 12, dst);
        memPutLong(p + 16, dstOffset);
        memPutLong(p + 24, size);
    }

    public void fillBuffer(int buffer, long offset, long size, byte value) {
        var p = head(7, 21);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutLong(p + 12, size);
        memPutByte(p + 20, value);
    }

    public void barrier(int srcStage, int srcAccess, int dstStage, int dstAccess) {
        var p = head(8, 16);
        memPutInt(p, srcStage);
        memPutInt(p + 4, srcAccess);
        memPutInt(p + 8, dstStage);
        memPutInt(p + 12, dstAccess);
    }

    public void beginLabel(String text) {
        var b = text.getBytes(StandardCharsets.UTF_8);
        var p = head(9, b.length);
        memByteBuffer(p, b.length).put(b);
    }

    public void endLabel() {
        head(10, 0);
    }

    public void queueBarrier(int srcStage, int srcAccess, int dstStage, int dstAccess) {
        var p = head(13, 16);
        memPutInt(p, srcStage);
        memPutInt(p + 4, srcAccess);
        memPutInt(p + 8, dstStage);
        memPutInt(p + 12, dstAccess);
    }

    public void textureBarrier(int texture, int srcStage, int srcAccess, int dstStage, int dstAccess, int old, int new, int baseMip, int mips, int baseLayer, int layers) {
        var p = head(14, 44);
        memPutInt(p, texture);
        memPutInt(p + 4, srcStage);
        memPutInt(p + 8, srcAccess);
        memPutInt(p + 12, dstStage);
        memPutInt(p + 16, dstAccess);
        memPutInt(p + 20, old);
        memPutInt(p + 24, new);
        memPutInt(p + 28, baseMip);
        memPutInt(p + 32, mips);
        memPutInt(p + 36, baseLayer);
        memPutInt(p + 40, layers);
    }

    public void copyBufferToTexture(int buffer, long offset, int rowLength, int rowCount, int texture, int layout, int mip, int layer, int x, int y, int z, int width, int height, int depth) {
        var p = head(15, 60);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, rowLength);
        memPutInt(p + 16, rowCount);
        memPutInt(p + 20, texture);
        memPutInt(p + 24, layout);
        memPutInt(p + 28, mip);
        memPutInt(p + 32, layer);
        memPutInt(p + 36, x);
        memPutInt(p + 40, y);
        memPutInt(p + 44, z);
        memPutInt(p + 48, width);
        memPutInt(p + 52, height);
        memPutInt(p + 56, depth);
    }

    public void copyTextureToBuffer(int texture, int layout, int mip, int layer, int x, int y, int z, int width, int height, int depth, int buffer, long offset, int rowLength, int rowCount) {
        var p = head(16, 60);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, mip);
        memPutInt(p + 12, layer);
        memPutInt(p + 16, x);
        memPutInt(p + 20, y);
        memPutInt(p + 24, z);
        memPutInt(p + 28, width);
        memPutInt(p + 32, height);
        memPutInt(p + 36, depth);
        memPutInt(p + 40, buffer);
        memPutLong(p + 44, offset);
        memPutInt(p + 52, rowLength);
        memPutInt(p + 56, rowCount);
    }
}
