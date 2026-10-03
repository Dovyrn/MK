package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import java.lang.ref.Cleaner;
import lombok.Getter;

public class Commands implements AutoCloseable {
    private static final Cleaner cleaner = Cleaner.create();
    private final Block block = new Block();
    private final Cleaner.Cleanable cleanable = cleaner.register(this, block);
    private int capacity = 4096;
    @Getter
    private int size;

    public long getAddress() {
        return block.address;
    }

    public void reset() {
        size = 0;
    }

    @Override
    public void close() {
        cleanable.clean();
    }

    protected long head(int op, int length) {
        if (block.address == 0) {
            throw new IllegalStateException("commands used after close");
        }
        if (length < 0 || length > 0xffff) {
            throw new IllegalArgumentException("payload of " + length + " bytes is outside 0..65535");
        }
        var needed = (long) size + 4 + length;
        if (needed > Integer.MAX_VALUE) {
            throw new IllegalStateException("commands are over 2 GiB");
        }
        if (needed > capacity) {
            capacity = (int) Math.min(Integer.MAX_VALUE, Math.max(2L * capacity, needed));
            block.address = nmemRealloc(block.address, capacity);
        }
        var p = block.address + size;
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

    public void pushConstants(int offset, long data, long dataSize) {
        if (dataSize < 0 || dataSize > 0xffff) {
            throw new IllegalArgumentException("dataSize " + dataSize + " is outside 0..65535");
        }
        var p = head(3, 4 + (int) dataSize);
        memPutInt(p, offset);
        memCopy(data, p + 4, dataSize);
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

    public void setLineWidth(float width) {
        var p = head(12, 4);
        memPutFloat(p, width);
    }

    public void queueBarrier(int srcStage, int srcAccess, int dstStage, int dstAccess) {
        var p = head(13, 16);
        memPutInt(p, srcStage);
        memPutInt(p + 4, srcAccess);
        memPutInt(p + 8, dstStage);
        memPutInt(p + 12, dstAccess);
    }

    public void textureBarrier(TextureBarrier info) {
        var p = head(14, 44);
        memPutInt(p, info.texture);
        memPutInt(p + 4, info.srcStage);
        memPutInt(p + 8, info.srcAccess);
        memPutInt(p + 12, info.dstStage);
        memPutInt(p + 16, info.dstAccess);
        memPutInt(p + 20, info.before);
        memPutInt(p + 24, info.after);
        memPutInt(p + 28, info.baseMip);
        memPutInt(p + 32, info.mips);
        memPutInt(p + 36, info.baseLayer);
        memPutInt(p + 40, info.layers);
    }

    public void copyBufferToTexture(TextureCopy info) {
        var p = head(15, 60);
        memPutInt(p, info.buffer);
        memPutLong(p + 4, info.offset);
        memPutInt(p + 12, info.rowLength);
        memPutInt(p + 16, info.rowCount);
        memPutInt(p + 20, info.texture);
        memPutInt(p + 24, info.layout);
        memPutInt(p + 28, info.mip);
        memPutInt(p + 32, info.layer);
        memPutInt(p + 36, info.x);
        memPutInt(p + 40, info.y);
        memPutInt(p + 44, info.z);
        memPutInt(p + 48, info.width);
        memPutInt(p + 52, info.height);
        memPutInt(p + 56, info.depth);
    }

    public void copyTextureToBuffer(TextureCopy info) {
        var p = head(16, 60);
        memPutInt(p, info.buffer);
        memPutLong(p + 4, info.offset);
        memPutInt(p + 12, info.rowLength);
        memPutInt(p + 16, info.rowCount);
        memPutInt(p + 20, info.texture);
        memPutInt(p + 24, info.layout);
        memPutInt(p + 28, info.mip);
        memPutInt(p + 32, info.layer);
        memPutInt(p + 36, info.x);
        memPutInt(p + 40, info.y);
        memPutInt(p + 44, info.z);
        memPutInt(p + 48, info.width);
        memPutInt(p + 52, info.height);
        memPutInt(p + 56, info.depth);
    }

    public void copyTexture(CopyInfo info) {
        var p = head(17, 72);
        memPutInt(p, info.src);
        memPutInt(p + 4, info.srcLayout);
        memPutInt(p + 8, info.srcMip);
        memPutInt(p + 12, info.srcLayer);
        memPutInt(p + 16, info.srcX);
        memPutInt(p + 20, info.srcY);
        memPutInt(p + 24, info.srcZ);
        memPutInt(p + 28, info.dst);
        memPutInt(p + 32, info.dstLayout);
        memPutInt(p + 36, info.dstMip);
        memPutInt(p + 40, info.dstLayer);
        memPutInt(p + 44, info.dstX);
        memPutInt(p + 48, info.dstY);
        memPutInt(p + 52, info.dstZ);
        memPutInt(p + 56, info.width);
        memPutInt(p + 60, info.height);
        memPutInt(p + 64, info.depth);
        memPutInt(p + 68, info.layers);
    }

    public void blitTexture(BlitInfo info) {
        var p = head(18, 68);
        memPutInt(p, info.src);
        memPutInt(p + 4, info.srcLayout);
        memPutInt(p + 8, info.srcMip);
        memPutInt(p + 12, info.srcLayer);
        memPutInt(p + 16, info.srcX);
        memPutInt(p + 20, info.srcY);
        memPutInt(p + 24, info.srcWidth);
        memPutInt(p + 28, info.srcHeight);
        memPutInt(p + 32, info.dst);
        memPutInt(p + 36, info.dstLayout);
        memPutInt(p + 40, info.dstMip);
        memPutInt(p + 44, info.dstLayer);
        memPutInt(p + 48, info.dstX);
        memPutInt(p + 52, info.dstY);
        memPutInt(p + 56, info.dstWidth);
        memPutInt(p + 60, info.dstHeight);
        memPutInt(p + 64, info.filter);
    }

    public void generateMips(int texture, int layout) {
        var p = head(19, 8);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
    }

    public void clearTexture(ClearInfo info) {
        var p = head(20, 48);
        memPutInt(p, info.texture);
        memPutInt(p + 4, info.layout);
        memPutInt(p + 8, info.baseMip);
        memPutInt(p + 12, info.mips);
        memPutInt(p + 16, info.baseLayer);
        memPutInt(p + 20, info.layers);
        memPutFloat(p + 24, info.r);
        memPutFloat(p + 28, info.g);
        memPutFloat(p + 32, info.b);
        memPutFloat(p + 36, info.a);
        memPutFloat(p + 40, info.depth);
        memPutInt(p + 44, info.stencil);
    }

    public void colorAttachment(ColorAttachment info) {
        var p = head(21, 44);
        memPutInt(p, info.index);
        memPutInt(p + 4, info.texture);
        memPutInt(p + 8, info.layout);
        memPutInt(p + 12, info.load);
        memPutInt(p + 16, info.store);
        memPutFloat(p + 20, info.r);
        memPutFloat(p + 24, info.g);
        memPutFloat(p + 28, info.b);
        memPutFloat(p + 32, info.a);
        memPutInt(p + 36, info.resolve);
        memPutInt(p + 40, info.resolveLayout);
    }

    public void depthAttachment(DepthAttachment info) {
        var p = head(22, 32);
        memPutInt(p, info.texture);
        memPutInt(p + 4, info.layout);
        memPutInt(p + 8, info.load);
        memPutInt(p + 12, info.store);
        memPutFloat(p + 16, info.depth);
        memPutInt(p + 20, info.stencilLoad);
        memPutInt(p + 24, info.stencilStore);
        memPutInt(p + 28, info.stencil);
    }

    public void beginRender(int width, int height, int layers) {
        var p = head(23, 12);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, layers);
    }

    public void endRender() {
        head(24, 0);
    }

    public void bindRenderPipeline(int pipeline) {
        var p = head(25, 4);
        memPutInt(p, pipeline);
    }

    public void setViewport(Viewport info) {
        var p = head(26, 24);
        memPutFloat(p, info.x);
        memPutFloat(p + 4, info.y);
        memPutFloat(p + 8, info.width);
        memPutFloat(p + 12, info.height);
        memPutFloat(p + 16, info.near);
        memPutFloat(p + 20, info.far);
    }

    public void setScissor(int x, int y, int width, int height) {
        var p = head(27, 16);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, width);
        memPutInt(p + 12, height);
    }

    public void setCull(int cull) {
        var p = head(28, 4);
        memPutInt(p, cull);
    }

    public void setFace(int face) {
        var p = head(29, 4);
        memPutInt(p, face);
    }

    public void setTopology(int topology) {
        var p = head(30, 4);
        memPutInt(p, topology);
    }

    public void setDepthState(int state) {
        var p = head(31, 4);
        memPutInt(p, state);
    }

    public void setStencilReference(int front, int back) {
        var p = head(32, 8);
        memPutInt(p, front);
        memPutInt(p + 4, back);
    }

    public void setDepthBias(float constant, float slope, float clamp) {
        var p = head(33, 12);
        memPutFloat(p, constant);
        memPutFloat(p + 4, slope);
        memPutFloat(p + 8, clamp);
    }

    public void setBlendColor(float r, float g, float b, float a) {
        var p = head(34, 16);
        memPutFloat(p, r);
        memPutFloat(p + 4, g);
        memPutFloat(p + 8, b);
        memPutFloat(p + 12, a);
    }

    public void bindUniform(int slot, int buffer, long offset, long size) {
        var p = head(35, 24);
        memPutInt(p, slot);
        memPutInt(p + 4, buffer);
        memPutLong(p + 8, offset);
        memPutLong(p + 16, size);
    }

    public void bindStorage(int slot, int buffer, long offset, long size) {
        var p = head(36, 24);
        memPutInt(p, slot);
        memPutInt(p + 4, buffer);
        memPutLong(p + 8, offset);
        memPutLong(p + 16, size);
    }

    public void bindTexture(int slot, int texture) {
        var p = head(37, 8);
        memPutInt(p, slot);
        memPutInt(p + 4, texture);
    }

    public void bindImage(int slot, int texture) {
        var p = head(38, 8);
        memPutInt(p, slot);
        memPutInt(p + 4, texture);
    }

    public void bindSampler(int slot, int sampler) {
        var p = head(39, 8);
        memPutInt(p, slot);
        memPutInt(p + 4, sampler);
    }

    public void bindVertexBuffer(int slot, int buffer, long offset) {
        var p = head(40, 16);
        memPutInt(p, slot);
        memPutInt(p + 4, buffer);
        memPutLong(p + 8, offset);
    }

    public void bindIndexBuffer(int buffer, long offset, int kind) {
        var p = head(41, 16);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, kind);
    }

    public void draw(int vertices, int instances, int firstVertex, int firstInstance) {
        var p = head(42, 16);
        memPutInt(p, vertices);
        memPutInt(p + 4, instances);
        memPutInt(p + 8, firstVertex);
        memPutInt(p + 12, firstInstance);
    }

    public void drawIndexed(int indices, int instances, int firstIndex, int baseVertex, int firstInstance) {
        var p = head(43, 20);
        memPutInt(p, indices);
        memPutInt(p + 4, instances);
        memPutInt(p + 8, firstIndex);
        memPutInt(p + 12, baseVertex);
        memPutInt(p + 16, firstInstance);
    }

    public void drawIndirect(int buffer, long offset, int count, int stride) {
        var p = head(44, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void drawIndexedIndirect(int buffer, long offset, int count, int stride) {
        var p = head(45, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void drawIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(46, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void drawIndexedIndirectCount(int buffer, long offset, int count, long countOffset, int max, int stride) {
        var p = head(47, 32);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutLong(p + 16, countOffset);
        memPutInt(p + 24, max);
        memPutInt(p + 28, stride);
    }

    public void drawMesh(int x, int y, int z) {
        var p = head(48, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }

    public void drawMeshIndirect(int buffer, long offset, int count, int stride) {
        var p = head(49, 20);
        memPutInt(p, buffer);
        memPutLong(p + 4, offset);
        memPutInt(p + 12, count);
        memPutInt(p + 16, stride);
    }

    public void occlusionHeap(int heap) {
        var p = head(50, 4);
        memPutInt(p, heap);
    }

    public void resetQueries(int heap, int first, int count) {
        var p = head(51, 12);
        memPutInt(p, heap);
        memPutInt(p + 4, first);
        memPutInt(p + 8, count);
    }

    public void writeTimestamp(int heap, int index, int stage) {
        var p = head(52, 12);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
        memPutInt(p + 8, stage);
    }

    public void beginQuery(int heap, int index) {
        var p = head(53, 8);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
    }

    public void endQuery(int heap, int index) {
        var p = head(54, 8);
        memPutInt(p, heap);
        memPutInt(p + 4, index);
    }

    public void resolveQueries(int heap, int first, int count, int buffer, long offset) {
        var p = head(55, 24);
        memPutInt(p, heap);
        memPutInt(p + 4, first);
        memPutInt(p + 8, count);
        memPutInt(p + 12, buffer);
        memPutLong(p + 16, offset);
    }

    public void multiview(int views) {
        var p = head(60, 4);
        memPutInt(p, views);
    }

    public void setViewports(Viewport[] viewports) {
        var p = head(73, viewports.length * 24);
        for (var i = 0; i < viewports.length; i++) {
            var q = p + i * 24;
            memPutFloat(q, viewports[i].x);
            memPutFloat(q + 4, viewports[i].y);
            memPutFloat(q + 8, viewports[i].width);
            memPutFloat(q + 12, viewports[i].height);
            memPutFloat(q + 16, viewports[i].near);
            memPutFloat(q + 20, viewports[i].far);
        }
    }

    public void setScissors(Scissor[] scissors) {
        var p = head(74, scissors.length * 16);
        for (var i = 0; i < scissors.length; i++) {
            var q = p + i * 16;
            memPutInt(q, scissors[i].x);
            memPutInt(q + 4, scissors[i].y);
            memPutInt(q + 8, scissors[i].width);
            memPutInt(q + 12, scissors[i].height);
        }
    }

    public void setDepthBounds(float min, float max) {
        var p = head(75, 8);
        memPutFloat(p, min);
        memPutFloat(p + 4, max);
    }
}
