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
}
