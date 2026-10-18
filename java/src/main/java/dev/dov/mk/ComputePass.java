package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class ComputePass {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    ComputePass(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording.incrementAndGet();
        }
    }

    static ComputePass adopt(Device device, long handle) {
        return new ComputePass(device, null, handle);
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("pass used after end");
        }
        return handle;
    }

    public void end() {
        var ended = handle();
        handle = 0;
        nEnd(ended);
        if (owner != null) {
            owner.ended();
        } else {
            device.recording.decrementAndGet();
        }
    }

    public long raw() {
        return nRaw(handle());
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);

    public void bindComputePipeline(ComputePipeline pipeline) {
        nBindComputePipeline(handle(), ComputePipeline.raw(pipeline));
    }

    private static native void nBindComputePipeline(long target, long pipeline);

    public void dispatch(int x, int y, int z) {
        nDispatch(handle(), x, y, z);
    }

    private static native void nDispatch(long target, int x, int y, int z);

    public void dispatchIndirect(Buffer buffer, long offset) {
        nDispatchIndirect(handle(), Buffer.raw(buffer), offset);
    }

    private static native void nDispatchIndirect(long target, long buffer, long offset);

    public void copyBuffer(Buffer src, long srcOffset, Buffer dst, long dstOffset, long size) {
        nCopyBuffer(handle(), Buffer.raw(src), srcOffset, Buffer.raw(dst), dstOffset, size);
    }

    private static native void nCopyBuffer(long target, long src, long srcOffset, long dst, long dstOffset, long size);

    public void fillBuffer(Buffer buffer, long offset, long size, int value) {
        nFillBuffer(handle(), Buffer.raw(buffer), offset, size, value & 0xff);
    }

    private static native void nFillBuffer(long target, long buffer, long offset, long size, int value);

    public void memoryBarrier(int srcStage, int dstStage) {
        nMemoryBarrier(handle(), srcStage, dstStage);
    }

    private static native void nMemoryBarrier(long target, int srcStage, int dstStage);

    public void beginDebugLabel(String text) {
        try (var arena = new Arena()) {
            var encoded = text.getBytes(StandardCharsets.UTF_8);
            nBeginDebugLabel(handle(), arena.bytes(encoded), encoded.length);
        }
    }

    private static native void nBeginDebugLabel(long target, long text, int textLength);

    public void endDebugLabel() {
        nEndDebugLabel(handle());
    }

    private static native void nEndDebugLabel(long target);

    public void presentBarrier(Texture texture, int srcStage) {
        nPresentBarrier(handle(), Texture.raw(texture), srcStage);
    }

    private static native void nPresentBarrier(long target, long texture, int srcStage);

    public void acquireShared(Texture texture, int dstStage) {
        nAcquireShared(handle(), Texture.raw(texture), dstStage);
    }

    private static native void nAcquireShared(long target, long texture, int dstStage);

    public void releaseShared(Texture texture, int srcStage) {
        nReleaseShared(handle(), Texture.raw(texture), srcStage);
    }

    private static native void nReleaseShared(long target, long texture, int srcStage);

    public void discardTextures(TextureDiscard[] textures) {
        try (var arena = new Arena()) {
            var texturesSize = 4;
            for (var x : textures) {
                texturesSize += x.encodedSize();
            }
            var list = arena.alloc(texturesSize);
            var at = list;
            memPutInt(at, textures.length);
            at += 4;
            for (var x : textures) {
                at = x.encode(at);
            }
            nDiscardTextures(handle(), list, texturesSize);
        }
    }

    private static native void nDiscardTextures(long target, long textures, int texturesLength);

    public void copyBufferToTexture(BufferTextureCopy info) {
        nCopyBufferToTexture(handle(), Buffer.raw(info.buffer), info.offset, info.rowLength, info.rowCount, Texture.raw(info.texture), info.mip, info.layer, info.x, info.y, info.z, info.width, info.height, info.depth);
    }

    private static native void nCopyBufferToTexture(long target, long buffer, long offset, int rowLength, int rowCount, long texture, int mip, int layer, int x, int y, int z, int width, int height, int depth);

    public void copyTextureToBuffer(BufferTextureCopy info) {
        nCopyTextureToBuffer(handle(), Buffer.raw(info.buffer), info.offset, info.rowLength, info.rowCount, Texture.raw(info.texture), info.mip, info.layer, info.x, info.y, info.z, info.width, info.height, info.depth);
    }

    private static native void nCopyTextureToBuffer(long target, long buffer, long offset, int rowLength, int rowCount, long texture, int mip, int layer, int x, int y, int z, int width, int height, int depth);

    public void copyTexture(TextureCopy info) {
        nCopyTexture(handle(), Texture.raw(info.src), info.srcMip, info.srcLayer, info.srcX, info.srcY, info.srcZ, Texture.raw(info.dst), info.dstMip, info.dstLayer, info.dstX, info.dstY, info.dstZ, info.width, info.height, info.depth, info.layers);
    }

    private static native void nCopyTexture(long target, long src, int srcMip, int srcLayer, int srcX, int srcY, int srcZ, long dst, int dstMip, int dstLayer, int dstX, int dstY, int dstZ, int width, int height, int depth, int layers);

    public void bindDescriptorSet(DescriptorSet set) {
        nBindDescriptorSet(handle(), DescriptorSet.raw(set));
    }

    private static native void nBindDescriptorSet(long target, long set);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void resolveOcclusion(QueryPool pool, int first, int count, Buffer buffer, long offset) {
        nResolveOcclusion(handle(), QueryPool.raw(pool), first, count, Buffer.raw(buffer), offset);
    }

    private static native void nResolveOcclusion(long target, long pool, int first, int count, long buffer, long offset);
}
