package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class ComputePass implements AutoCloseable {
    final Device device;
    private final CommandBuffer owner;
    private long handle;

    ComputePass(Device device, CommandBuffer owner, long handle) {
        this.device = device;
        this.owner = owner;
        this.handle = handle;
        if (owner == null) {
            device.recording++;
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
        synchronized (device) {
            var ended = handle();
            handle = 0;
            if (owner != null) {
                owner.ended();
            } else {
                device.recording--;
            }
            nEnd(ended);
        }
    }

    public long raw() {
        synchronized (device) {
            return nRaw(handle());
        }
    }

    @Override
    public void close() {
        if (handle != 0) {
            end();
        }
    }

    private static native void nEnd(long pass);

    private static native long nRaw(long pass);

    public void bindComputePipeline(ComputePipeline pipeline) {
        synchronized (device) {
            nBindComputePipeline(handle(), ComputePipeline.raw(pipeline));
        }
    }

    private static native void nBindComputePipeline(long target, long pipeline);

    public void dispatch(int x, int y, int z) {
        synchronized (device) {
            nDispatch(handle(), x, y, z);
        }
    }

    private static native void nDispatch(long target, int x, int y, int z);

    public void dispatchIndirect(Buffer buffer, long offset) {
        synchronized (device) {
            nDispatchIndirect(handle(), Buffer.raw(buffer), offset);
        }
    }

    private static native void nDispatchIndirect(long target, long buffer, long offset);

    public void copyBuffer(Buffer src, long srcOffset, Buffer dst, long dstOffset, long size) {
        synchronized (device) {
            nCopyBuffer(handle(), Buffer.raw(src), srcOffset, Buffer.raw(dst), dstOffset, size);
        }
    }

    private static native void nCopyBuffer(long target, long src, long srcOffset, long dst, long dstOffset, long size);

    public void fillBuffer(Buffer buffer, long offset, long size, int value) {
        synchronized (device) {
            nFillBuffer(handle(), Buffer.raw(buffer), offset, size, value & 0xff);
        }
    }

    private static native void nFillBuffer(long target, long buffer, long offset, long size, int value);

    public void memoryBarrier(int srcStage, int dstStage) {
        synchronized (device) {
            nMemoryBarrier(handle(), srcStage, dstStage);
        }
    }

    private static native void nMemoryBarrier(long target, int srcStage, int dstStage);

    public void beginDebugLabel(String text) {
        synchronized (device) {
            try (var arena = new Arena()) {
            var encoded = text.getBytes(StandardCharsets.UTF_8);
                nBeginDebugLabel(handle(), arena.bytes(encoded), encoded.length);
            }
        }
    }

    private static native void nBeginDebugLabel(long target, long text, int textLength);

    public void endDebugLabel() {
        synchronized (device) {
            nEndDebugLabel(handle());
        }
    }

    private static native void nEndDebugLabel(long target);

    public void textureBarrier(TextureBarrier info) {
        synchronized (device) {
            nTextureBarrier(handle(), Texture.raw(info.texture), info.srcStage, info.dstStage, info.before, info.after, info.baseMip, info.mips, info.baseLayer, info.layers);
        }
    }

    private static native void nTextureBarrier(long target, long texture, int srcStage, int dstStage, int before, int after, int baseMip, int mips, int baseLayer, int layers);

    public void copyBufferToTexture(BufferTextureCopy info) {
        synchronized (device) {
            nCopyBufferToTexture(handle(), Buffer.raw(info.buffer), info.offset, info.rowLength, info.rowCount, Texture.raw(info.texture), info.mip, info.layer, info.x, info.y, info.z, info.width, info.height, info.depth);
        }
    }

    private static native void nCopyBufferToTexture(long target, long buffer, long offset, int rowLength, int rowCount, long texture, int mip, int layer, int x, int y, int z, int width, int height, int depth);

    public void copyTextureToBuffer(BufferTextureCopy info) {
        synchronized (device) {
            nCopyTextureToBuffer(handle(), Buffer.raw(info.buffer), info.offset, info.rowLength, info.rowCount, Texture.raw(info.texture), info.mip, info.layer, info.x, info.y, info.z, info.width, info.height, info.depth);
        }
    }

    private static native void nCopyTextureToBuffer(long target, long buffer, long offset, int rowLength, int rowCount, long texture, int mip, int layer, int x, int y, int z, int width, int height, int depth);

    public void copyTexture(TextureCopy info) {
        synchronized (device) {
            nCopyTexture(handle(), Texture.raw(info.src), info.srcMip, info.srcLayer, info.srcX, info.srcY, info.srcZ, Texture.raw(info.dst), info.dstMip, info.dstLayer, info.dstX, info.dstY, info.dstZ, info.width, info.height, info.depth, info.layers);
        }
    }

    private static native void nCopyTexture(long target, long src, int srcMip, int srcLayer, int srcX, int srcY, int srcZ, long dst, int dstMip, int dstLayer, int dstX, int dstY, int dstZ, int width, int height, int depth, int layers);

    public void bindUniformBuffer(int slot, Buffer buffer, long offset, long size) {
        synchronized (device) {
            nBindUniformBuffer(handle(), slot, Buffer.raw(buffer), offset, size);
        }
    }

    private static native void nBindUniformBuffer(long target, int slot, long buffer, long offset, long size);

    public void bindStorageBuffer(int slot, Buffer buffer, long offset, long size) {
        synchronized (device) {
            nBindStorageBuffer(handle(), slot, Buffer.raw(buffer), offset, size);
        }
    }

    private static native void nBindStorageBuffer(long target, int slot, long buffer, long offset, long size);

    public void bindTexture(int slot, Texture texture) {
        synchronized (device) {
            nBindTexture(handle(), slot, Texture.raw(texture));
        }
    }

    private static native void nBindTexture(long target, int slot, long texture);

    public void bindStorageTexture(int slot, Texture texture) {
        synchronized (device) {
            nBindStorageTexture(handle(), slot, Texture.raw(texture));
        }
    }

    private static native void nBindStorageTexture(long target, int slot, long texture);

    public void bindSampler(int slot, Sampler sampler) {
        synchronized (device) {
            nBindSampler(handle(), slot, Sampler.raw(sampler));
        }
    }

    private static native void nBindSampler(long target, int slot, long sampler);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        synchronized (device) {
            nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
        }
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void resolveOcclusion(QueryPool pool, int first, int count, Buffer buffer, long offset) {
        synchronized (device) {
            nResolveOcclusion(handle(), QueryPool.raw(pool), first, count, Buffer.raw(buffer), offset);
        }
    }

    private static native void nResolveOcclusion(long target, long pool, int first, int count, long buffer, long offset);
}
