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

    public void textureBarrier(int texture, int srcStage, int srcAccess, int dstStage, int dstAccess, int before, int after, int baseMip, int mips, int baseLayer, int layers) {
        var p = head(14, 44);
        memPutInt(p, texture);
        memPutInt(p + 4, srcStage);
        memPutInt(p + 8, srcAccess);
        memPutInt(p + 12, dstStage);
        memPutInt(p + 16, dstAccess);
        memPutInt(p + 20, before);
        memPutInt(p + 24, after);
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

    public void colorAttachment(int index, int texture, int layout, int load, int store, float r, float g, float b, float a, int resolve, int resolveLayout) {
        var p = head(17, 44);
        memPutInt(p, index);
        memPutInt(p + 4, texture);
        memPutInt(p + 8, layout);
        memPutInt(p + 12, load);
        memPutInt(p + 16, store);
        memPutFloat(p + 20, r);
        memPutFloat(p + 24, g);
        memPutFloat(p + 28, b);
        memPutFloat(p + 32, a);
        memPutInt(p + 36, resolve);
        memPutInt(p + 40, resolveLayout);
    }

    public void depthAttachment(int texture, int layout, int load, int store, float depth, int stencilLoad, int stencilStore, int stencil) {
        var p = head(18, 32);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, load);
        memPutInt(p + 12, store);
        memPutFloat(p + 16, depth);
        memPutInt(p + 20, stencilLoad);
        memPutInt(p + 24, stencilStore);
        memPutInt(p + 28, stencil);
    }

    public void beginRender(int width, int height, int layers) {
        var p = head(19, 12);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, layers);
    }

    public void endRender() {
        head(20, 0);
    }

    public void bindRenderPipeline(int pipeline) {
        var p = head(21, 4);
        memPutInt(p, pipeline);
    }

    public void setViewport(float x, float y, float width, float height, float near, float far) {
        var p = head(22, 24);
        memPutFloat(p, x);
        memPutFloat(p + 4, y);
        memPutFloat(p + 8, width);
        memPutFloat(p + 12, height);
        memPutFloat(p + 16, near);
        memPutFloat(p + 20, far);
    }

    public void setScissor(int x, int y, int width, int height) {
        var p = head(23, 16);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, width);
        memPutInt(p + 12, height);
    }

    public void setCull(int cull) {
        var p = head(24, 4);
        memPutInt(p, cull);
    }

    public void setFace(int face) {
        var p = head(25, 4);
        memPutInt(p, face);
    }

    public void setTopology(int topology) {
        var p = head(26, 4);
        memPutInt(p, topology);
    }

    public void setDepthState(int state) {
        var p = head(27, 4);
        memPutInt(p, state);
    }

    public void setStencilReference(int front, int back) {
        var p = head(28, 8);
        memPutInt(p, front);
        memPutInt(p + 4, back);
    }

    public void setDepthBias(float constant, float slope, float clamp) {
        var p = head(29, 12);
        memPutFloat(p, constant);
        memPutFloat(p + 4, slope);
        memPutFloat(p + 8, clamp);
    }

    public void setBlendColor(float r, float g, float b, float a) {
        var p = head(30, 16);
        memPutFloat(p, r);
        memPutFloat(p + 4, g);
        memPutFloat(p + 8, b);
        memPutFloat(p + 12, a);
    }

    public void bindVertexBuffer(int slot, int buffer, long offset) {
        var p = head(31, 16);
        memPutInt(p, slot);
        memPutInt(p + 4, buffer);
        memPutLong(p + 8, offset);
    }

    public void bindIndexBuffer(int buffer, long offset, int kind) {
        var p = head(32, 16);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, kind);
    }

    public void draw(int vertices, int instances, int firstVertex, int firstInstance) {
        var p = head(33, 16);
        memPutInt(p, vertices);
        memPutInt(p + 4, instances);
        memPutInt(p + 8, firstVertex);
        memPutInt(p + 12, firstInstance);
    }

    public void drawIndexed(int indices, int instances, int firstIndex, int baseVertex, int firstInstance) {
        var p = head(34, 20);
        memPutInt(p, indices);
        memPutInt(p + 4, instances);
        memPutInt(p + 8, firstIndex);
        memPutInt(p + 12, baseVertex);
        memPutInt(p + 16, firstInstance);
    }

    public void drawIndirect(int buffer, long offset, int count, int stride) {
        var p = head(35, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void drawIndexedIndirect(int buffer, long offset, int count, int stride) {
        var p = head(36, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void drawMesh(int x, int y, int z) {
        var p = head(37, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }

    public void drawMeshIndirect(int buffer, long offset, int count, int stride) {
        var p = head(38, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void occlusionHeap(int heap) {
        var p = head(39, 4);
        memPutInt(p, heap);
    }

    public void resetQueries(int heap, int first, int count) {
        var p = head(40, 12);
        memPutInt(p, heap);
        memPutInt(p + 4, first);
        memPutInt(p + 8, count);
    }

    public void writeTimestamp(int heap, int index, int stage) {
        var p = head(41, 12);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
        memPutInt(p + 8, stage);
    }

    public void beginQuery(int heap, int index) {
        var p = head(42, 8);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
    }

    public void endQuery(int heap, int index) {
        var p = head(43, 8);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
    }

    public void resolveQueries(int heap, int first, int count, int buffer, long offset) {
        var p = head(44, 24);
        memPutInt(p, heap);
        memPutInt(p + 4, first);
        memPutInt(p + 8, count);
        memPutInt(p + 12, buffer);
        memPutLong(p + 16, offset);
    }

    public void buildAccel(int accel, int scratch, long scratchOffset, int update) {
        var p = head(45, 20);
        memPutInt(p, accel);
        memPutInt(p + 4, scratch);
        memPutLong(p + 8, scratchOffset);
        memPutInt(p + 16, update);
    }

    public void multiview(int views) {
        var p = head(49, 4);
        memPutInt(p, views);
    }

    public void writeCompactedSize(int accel, int heap, int index) {
        var p = head(60, 12);
        memPutInt(p, accel);
        memPutInt(p + 4, heap);
        memPutInt(p + 8, index);
    }

    public void copyAccel(int src, int dst, int compact) {
        var p = head(61, 12);
        memPutInt(p, src);
        memPutInt(p + 4, dst);
        memPutInt(p + 8, compact);
    }

    public void setViewports(ByteBuffer data) {
        var p = head(63, data.remaining());
        memCopy(memAddress(data), p, data.remaining());
    }

    public void setScissors(ByteBuffer data) {
        var p = head(64, data.remaining());
        memCopy(memAddress(data), p, data.remaining());
    }

    public void setDepthBounds(float min, float max) {
        var p = head(65, 8);
        memPutFloat(p, min);
        memPutFloat(p + 4, max);
    }
}
