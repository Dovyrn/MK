package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class CommandBuffer implements AutoCloseable {
    final Device device;
    private long handle;
    private boolean passing;

    CommandBuffer(Device device, long handle) {
        this.device = device;
        this.handle = handle;
        device.recording.incrementAndGet();
    }

    static CommandBuffer adopt(Device device, long handle) {
        return new CommandBuffer(device, handle);
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("command buffer used after submit or close");
        }
        if (passing) {
            throw new IllegalStateException("command buffer used while one of its passes is open; end the pass first");
        }
        return handle;
    }

    long take() {
        var taken = handle();
        handle = 0;
        device.recording.decrementAndGet();
        return taken;
    }

    void ended() {
        passing = false;
    }

    public ComputePass beginComputePass(int srcStage, int dstStage) {
        var pass = nBeginComputePass(handle(), srcStage, dstStage);
        passing = true;
        return new ComputePass(device, this, pass);
    }

    public RenderPass beginRenderPass(RenderPassInfo info) {
        try (var arena = new Arena()) {
            var size = info.encodedSize();
            var address = arena.alloc(size);
            info.encode(address);
            var pass = nBeginRenderPass(handle(), address, size);
            passing = true;
            return new RenderPass(device, this, pass);
        }
    }

    public void end() {
        nEnd(handle());
    }

    public long raw() {
        return nRaw(handle());
    }

    @Override
    public void close() {
        if (handle != 0) {
            var dropped = handle;
            handle = 0;
            nDrop(dropped);
            device.recording.decrementAndGet();
        }
    }

    private static native long nBeginComputePass(long commands, int srcStage, int dstStage);

    private static native long nBeginRenderPass(long commands, long info, int infoLength);

    private static native void nEnd(long commands);

    private static native long nRaw(long commands);

    private static native void nDrop(long commands);

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

    public void useResidencySet(ResidencySet set) {
        nUseResidencySet(handle(), ResidencySet.raw(set));
    }

    private static native void nUseResidencySet(long target, long set);

    public void resetQueries(QueryPool pool, int first, int count) {
        nResetQueries(handle(), QueryPool.raw(pool), first, count);
    }

    private static native void nResetQueries(long target, long pool, int first, int count);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void resolveTimestamps(QueryPool pool, int first, int count, Buffer buffer, long offset) {
        nResolveTimestamps(handle(), QueryPool.raw(pool), first, count, Buffer.raw(buffer), offset);
    }

    private static native void nResolveTimestamps(long target, long pool, int first, int count, long buffer, long offset);
}
