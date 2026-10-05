package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class Device implements AutoCloseable {
    protected long handle;

    public Device(long handle) {
        this.handle = handle;
    }

    @Override
    public synchronized void close() {
        var device = handle;
        handle = 0;
        if (device != 0) {
            nFree(device);
        }
    }

    private static native void nFree(long device);

    public synchronized Caps caps() {
        try (var arena = new Arena()) {
            var out = arena.alloc(Caps.SIZE);
            nCaps(handle, out);
            return Caps.decode(out);
        }
    }

    private static native void nCaps(long device, long out);

    public synchronized String name() {
        return nName(handle);
    }

    private static native String nName(long device);

    public synchronized int createBuffer(long size, int memory) {
        return nCreateBuffer(handle, size, memory);
    }

    private static native int nCreateBuffer(long device, long size, int memory);

    public synchronized void destroyBuffer(int buffer) {
        nDestroyBuffer(handle, buffer);
    }

    private static native void nDestroyBuffer(long device, int buffer);

    public synchronized int createIndirectBuffer(IndirectInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateIndirectBuffer(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateIndirectBuffer(long device, long info, int infoLength);

    public synchronized void destroyIndirectBuffer(int indirect) {
        nDestroyIndirectBuffer(handle, indirect);
    }

    private static native void nDestroyIndirectBuffer(long device, int indirect);

    public synchronized int[] indirectHandle(int indirect) {
        try (var arena = new Arena()) {
            var out = arena.alloc(32);
            nIndirectHandle(handle, indirect, out);
            return new int[] {memGetInt(out + 0), memGetInt(out + 4), memGetInt(out + 8), memGetInt(out + 12), memGetInt(out + 16), memGetInt(out + 20), memGetInt(out + 24), memGetInt(out + 28)};
        }
    }

    private static native void nIndirectHandle(long device, int indirect, long out);

    public synchronized long map(int buffer) {
        return nMap(handle, buffer);
    }

    private static native long nMap(long device, int buffer);

    public synchronized long bufferHandle(int buffer) {
        return nBufferHandle(handle, buffer);
    }

    private static native long nBufferHandle(long device, int buffer);

    public synchronized void unmap(int buffer) {
        nUnmap(handle, buffer);
    }

    private static native void nUnmap(long device, int buffer);

    public synchronized long address(int buffer) {
        return nAddress(handle, buffer);
    }

    private static native long nAddress(long device, int buffer);

    public synchronized Matrix matrix(int index) {
        try (var arena = new Arena()) {
            var out = arena.alloc(Matrix.SIZE);
            nMatrix(handle, index, out);
            return Matrix.decode(out);
        }
    }

    private static native void nMatrix(long device, int index, long out);

    public synchronized Budget memoryBudget() {
        try (var arena = new Arena()) {
            var out = arena.alloc(Budget.SIZE);
            nMemoryBudget(handle, out);
            return Budget.decode(out);
        }
    }

    private static native void nMemoryBudget(long device, long out);

    public synchronized int createPipeline(ComputeInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreatePipeline(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreatePipeline(long device, long info, int infoLength);

    public synchronized void destroyPipeline(int pipeline) {
        nDestroyPipeline(handle, pipeline);
    }

    private static native void nDestroyPipeline(long device, int pipeline);

    public synchronized int createRenderPipeline(RenderInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateRenderPipeline(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateRenderPipeline(long device, long info, int infoLength);

    public synchronized int createMeshPipeline(MeshInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateMeshPipeline(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateMeshPipeline(long device, long info, int infoLength);

    public synchronized void destroyRenderPipeline(int pipeline) {
        nDestroyRenderPipeline(handle, pipeline);
    }

    private static native void nDestroyRenderPipeline(long device, int pipeline);

    public synchronized int createTilePipeline(TileInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateTilePipeline(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateTilePipeline(long device, long info, int infoLength);

    public synchronized void destroyTilePipeline(int pipeline) {
        nDestroyTilePipeline(handle, pipeline);
    }

    private static native void nDestroyTilePipeline(long device, int pipeline);

    public synchronized int createDepthState(DepthInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateDepthState(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateDepthState(long device, long info, int infoLength);

    public synchronized void destroyDepthState(int state) {
        nDestroyDepthState(handle, state);
    }

    private static native void nDestroyDepthState(long device, int state);

    public synchronized int createTexture(TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateTexture(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateTexture(long device, long info, int infoLength);

    public synchronized void writeTexture(HostCopy copy, long data, long dataSize) {
        try (var arena = new Arena()) {
            var copyAddress = arena.alloc(copy.encodedSize());
            copy.encode(copyAddress);
            nWriteTexture(handle, copyAddress, copy.encodedSize(), data, dataSize);
        }
    }

    private static native void nWriteTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized void readTexture(HostCopy copy, long data, long dataSize) {
        try (var arena = new Arena()) {
            var copyAddress = arena.alloc(copy.encodedSize());
            copy.encode(copyAddress);
            nReadTexture(handle, copyAddress, copy.encodedSize(), data, dataSize);
        }
    }

    private static native void nReadTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized int formatUsage(int format) {
        return nFormatUsage(handle, format);
    }

    private static native int nFormatUsage(long device, int format);

    public synchronized int formatSamples(int format, int usage) {
        return nFormatSamples(handle, format, usage);
    }

    private static native int nFormatSamples(long device, int format, int usage);

    public synchronized int createView(int texture, ViewInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateView(handle, texture, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateView(long device, int texture, long info, int infoLength);

    public synchronized void destroyTexture(int texture) {
        nDestroyTexture(handle, texture);
    }

    private static native void nDestroyTexture(long device, int texture);

    public synchronized long imageHandle(int texture) {
        return nImageHandle(handle, texture);
    }

    private static native long nImageHandle(long device, int texture);

    public synchronized int createSampler(SamplerInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateSampler(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateSampler(long device, long info, int infoLength);

    public synchronized void destroySampler(int sampler) {
        nDestroySampler(handle, sampler);
    }

    private static native void nDestroySampler(long device, int sampler);

    public synchronized int createQueryHeap(int kind, int count) {
        return nCreateQueryHeap(handle, kind, count);
    }

    private static native int nCreateQueryHeap(long device, int kind, int count);

    public synchronized void destroyQueryHeap(int heap) {
        nDestroyQueryHeap(handle, heap);
    }

    private static native void nDestroyQueryHeap(long device, int heap);

    public synchronized void recordCache() {
        nRecordCache(handle);
    }

    private static native void nRecordCache(long device);

    public synchronized void saveCache(String path) {
        try (var arena = new Arena()) {
            var pathBytes = path.getBytes(StandardCharsets.UTF_8);
            nSaveCache(handle, arena.bytes(pathBytes), pathBytes.length);
        }
    }

    private static native void nSaveCache(long device, long path, int pathLength);

    public synchronized void loadCache(String path) {
        try (var arena = new Arena()) {
            var pathBytes = path.getBytes(StandardCharsets.UTF_8);
            nLoadCache(handle, arena.bytes(pathBytes), pathBytes.length);
        }
    }

    private static native void nLoadCache(long device, long path, int pathLength);

    public synchronized Timeline createTimeline() {
        return new Timeline(nCreateTimeline(handle));
    }

    private static native long nCreateTimeline(long device);

    public synchronized void destroyTimeline(Timeline timeline) {
        nDestroyTimeline(handle, timeline.take());
    }

    private static native void nDestroyTimeline(long device, long timeline);

    public synchronized void submit(int queue, Commands commands, Point[] waits, Point[] signals) {
        try (var arena = new Arena()) {
            var waitsAddress = arena.alloc(waits.length * 16L);
            for (var i = 0; i < waits.length; i++) {
                memPutLong(waitsAddress + i * 16, waits[i].timeline().handle());
                memPutLong(waitsAddress + i * 16 + 8, waits[i].value());
            }
            var signalsAddress = arena.alloc(signals.length * 16L);
            for (var i = 0; i < signals.length; i++) {
                memPutLong(signalsAddress + i * 16, signals[i].timeline().handle());
                memPutLong(signalsAddress + i * 16 + 8, signals[i].value());
            }
            try {
                nSubmit(handle, queue, commands.getAddress(), commands.getSize(), waitsAddress, waits.length, signalsAddress, signals.length);
            } finally {
                java.lang.ref.Reference.reachabilityFence(commands);
            }
        }
    }

    private static native void nSubmit(long device, int queue, long commands, long commandsLength, long waits, int waitsLength, long signals, int signalsLength);
}
