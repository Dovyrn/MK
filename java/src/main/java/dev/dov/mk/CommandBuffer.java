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
        device.recording++;
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
        device.recording--;
        return taken;
    }

    void ended() {
        passing = false;
    }

    public ComputePass beginComputePass() {
        synchronized (device) {
            var pass = nBeginComputePass(handle());
            passing = true;
            return new ComputePass(device, this, pass);
        }
    }

    public RenderPass beginRenderPass(RenderPassInfo info) {
        synchronized (device) {
            try (var arena = new Arena()) {
                var size = info.encodedSize();
                var address = arena.alloc(size);
                info.encode(address);
                var pass = nBeginRenderPass(handle(), address, size);
                passing = true;
                return new RenderPass(device, this, pass);
            }
        }
    }

    public void end() {
        synchronized (device) {
            nEnd(handle());
        }
    }

    public long raw() {
        synchronized (device) {
            return nRaw(handle());
        }
    }

    @Override
    public void close() {
        synchronized (device) {
            if (handle != 0) {
                var dropped = handle;
                handle = 0;
                device.recording--;
                nDrop(dropped);
            }
        }
    }

    private static native long nBeginComputePass(long commands);

    private static native long nBeginRenderPass(long commands, long info, int infoLength);

    private static native void nEnd(long commands);

    private static native long nRaw(long commands);

    private static native void nDrop(long commands);

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

    public void textureBarriers(TextureBarrier[] barriers) {
        synchronized (device) {
            try (var arena = new Arena()) {
            var barriersSize = 4;
            for (var x : barriers) {
                barriersSize += x.encodedSize();
            }
            var list = arena.alloc(barriersSize);
            var at = list;
            memPutInt(at, barriers.length);
            at += 4;
            for (var x : barriers) {
                at = x.encode(at);
            }
                nTextureBarriers(handle(), list, barriersSize);
            }
        }
    }

    private static native void nTextureBarriers(long target, long barriers, int barriersLength);

    public void useResidencySet(ResidencySet set) {
        synchronized (device) {
            nUseResidencySet(handle(), ResidencySet.raw(set));
        }
    }

    private static native void nUseResidencySet(long target, long set);

    public void resetQueries(QueryPool pool, int first, int count) {
        synchronized (device) {
            nResetQueries(handle(), QueryPool.raw(pool), first, count);
        }
    }

    private static native void nResetQueries(long target, long pool, int first, int count);

    public void writeTimestamp(QueryPool pool, int index, int stage) {
        synchronized (device) {
            nWriteTimestamp(handle(), QueryPool.raw(pool), index, stage);
        }
    }

    private static native void nWriteTimestamp(long target, long pool, int index, int stage);

    public void resolveTimestamps(QueryPool pool, int first, int count, Buffer buffer, long offset) {
        synchronized (device) {
            nResolveTimestamps(handle(), QueryPool.raw(pool), first, count, Buffer.raw(buffer), offset);
        }
    }

    private static native void nResolveTimestamps(long target, long pool, int first, int count, long buffer, long offset);
}
