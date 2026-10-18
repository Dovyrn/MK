package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.locks.ReentrantReadWriteLock;

public class Device implements AutoCloseable {
    protected long handle;
    final AtomicInteger recording = new AtomicInteger();
    final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    public Device(long handle) {
        this.handle = handle;
    }

    long handle() {
        if (handle == 0) {
            throw new IllegalStateException("device used after close");
        }
        return handle;
    }

    void idle(String call) {
        var count = recording.get();
        if (count > 0) {
            throw new IllegalStateException(call + " changes the device while " + count
                    + " command buffers or imported passes are recording; submit or close them first");
        }
    }

    @Override
    public void close() {
        lock.writeLock().lock();
        try {
            idle("close");
            var device = handle;
            handle = 0;
            if (device != 0) {
                nFree(device);
            }
        } finally {
            lock.writeLock().unlock();
        }
    }

    private static native void nFree(long device);

    public Caps caps() {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var out = arena.alloc(Caps.SIZE);
                nCaps(handle(), out);
                return Caps.decode(out);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nCaps(long device, long out);

    public String deviceName() {
        lock.readLock().lock();
        try {
            return nDeviceName(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native String nDeviceName(long device);

    public Buffer createBuffer(long size, Memory memory, int usage, String label) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var labelBytes = label.getBytes(StandardCharsets.UTF_8);
                return Buffer.of(nCreateBuffer(handle(), size, memory.value, usage, arena.bytes(labelBytes), labelBytes.length));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateBuffer(long device, long size, int memory, int usage, long label, int labelLength);

    public void destroyBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            nDestroyBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyBuffer(long device, long buffer);

    public long mapBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            return nMapBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nMapBuffer(long device, long buffer);

    public void unmapBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            nUnmapBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nUnmapBuffer(long device, long buffer);

    public long bufferAddress(Buffer buffer) {
        lock.readLock().lock();
        try {
            return nBufferAddress(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nBufferAddress(long device, long buffer);

    public MemoryBudget memoryBudget() {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var out = arena.alloc(MemoryBudget.SIZE);
                nMemoryBudget(handle(), out);
                return MemoryBudget.decode(out);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nMemoryBudget(long device, long out);

    public ComputePipeline createComputePipeline(ComputePipelineInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return ComputePipeline.of(nCreateComputePipeline(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateComputePipeline(long device, long info, int infoLength);

    public void destroyComputePipeline(ComputePipeline pipeline) {
        lock.readLock().lock();
        try {
            nDestroyComputePipeline(handle(), ComputePipeline.raw(pipeline));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyComputePipeline(long device, long pipeline);

    public RenderPipeline createRenderPipeline(RenderPipelineInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return RenderPipeline.of(nCreateRenderPipeline(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateRenderPipeline(long device, long info, int infoLength);

    public RenderPipeline createMeshPipeline(MeshPipelineInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return RenderPipeline.of(nCreateMeshPipeline(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateMeshPipeline(long device, long info, int infoLength);

    public void destroyRenderPipeline(RenderPipeline pipeline) {
        lock.readLock().lock();
        try {
            nDestroyRenderPipeline(handle(), RenderPipeline.raw(pipeline));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyRenderPipeline(long device, long pipeline);

    public TilePipeline createTilePipeline(TilePipelineInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return TilePipeline.of(nCreateTilePipeline(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateTilePipeline(long device, long info, int infoLength);

    public void destroyTilePipeline(TilePipeline pipeline) {
        lock.readLock().lock();
        try {
            nDestroyTilePipeline(handle(), TilePipeline.raw(pipeline));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyTilePipeline(long device, long pipeline);

    public DepthState createDepthState(DepthStateInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return DepthState.of(nCreateDepthState(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateDepthState(long device, long info, int infoLength);

    public void destroyDepthState(DepthState state) {
        lock.readLock().lock();
        try {
            nDestroyDepthState(handle(), DepthState.raw(state));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyDepthState(long device, long state);

    public Texture createTexture(TextureInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Texture.of(nCreateTexture(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateTexture(long device, long info, int infoLength);

    public void writeTexture(HostTextureCopy copy, long data, long dataSize) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var copyAddress = arena.alloc(copy.encodedSize());
                copy.encode(copyAddress);
                nWriteTexture(handle(), copyAddress, copy.encodedSize(), data, dataSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nWriteTexture(long device, long copy, int copyLength, long data, long dataLength);

    public void readTexture(HostTextureCopy copy, long data, long dataSize) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var copyAddress = arena.alloc(copy.encodedSize());
                copy.encode(copyAddress);
                nReadTexture(handle(), copyAddress, copy.encodedSize(), data, dataSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nReadTexture(long device, long copy, int copyLength, long data, long dataLength);

    public int formatUsage(Format format) {
        lock.readLock().lock();
        try {
            return nFormatUsage(handle(), format.value);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native int nFormatUsage(long device, int format);

    public int formatSamples(Format format, int usage) {
        lock.readLock().lock();
        try {
            return nFormatSamples(handle(), format.value, usage);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native int nFormatSamples(long device, int format, int usage);

    public Texture createTextureView(Texture texture, TextureViewInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Texture.of(nCreateTextureView(handle(), Texture.raw(texture), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateTextureView(long device, long texture, long info, int infoLength);

    public void destroyTexture(Texture texture) {
        lock.readLock().lock();
        try {
            nDestroyTexture(handle(), Texture.raw(texture));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyTexture(long device, long texture);

    public long bindlessStorageTexture(Texture texture) {
        lock.readLock().lock();
        try {
            return nBindlessStorageTexture(handle(), Texture.raw(texture));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nBindlessStorageTexture(long device, long texture);

    public Sampler createSampler(SamplerInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Sampler.of(nCreateSampler(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateSampler(long device, long info, int infoLength);

    public void destroySampler(Sampler sampler) {
        lock.readLock().lock();
        try {
            nDestroySampler(handle(), Sampler.raw(sampler));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroySampler(long device, long sampler);

    public QueryPool createQueryPool(QueryKind kind, int count, String label) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var labelBytes = label.getBytes(StandardCharsets.UTF_8);
                return QueryPool.of(nCreateQueryPool(handle(), kind.value, count, arena.bytes(labelBytes), labelBytes.length));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateQueryPool(long device, int kind, int count, long label, int labelLength);

    public void destroyQueryPool(QueryPool pool) {
        lock.readLock().lock();
        try {
            nDestroyQueryPool(handle(), QueryPool.raw(pool));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyQueryPool(long device, long pool);

    public IndirectBuffer createIndirectBuffer(IndirectBufferInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return IndirectBuffer.of(nCreateIndirectBuffer(handle(), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateIndirectBuffer(long device, long info, int infoLength);

    public void destroyIndirectBuffer(IndirectBuffer indirect) {
        lock.readLock().lock();
        try {
            nDestroyIndirectBuffer(handle(), IndirectBuffer.raw(indirect));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyIndirectBuffer(long device, long indirect);

    public int[] indirectShaderHandle(IndirectBuffer indirect) {
        lock.writeLock().lock();
        try {
            idle("indirectShaderHandle");
            try (var arena = new Arena()) {
                var out = arena.alloc(32);
                nIndirectShaderHandle(handle(), IndirectBuffer.raw(indirect), out);
                return new int[] {memGetInt(out + 0), memGetInt(out + 4), memGetInt(out + 8), memGetInt(out + 12), memGetInt(out + 16), memGetInt(out + 20), memGetInt(out + 24), memGetInt(out + 28)};
            }
        } finally {
            lock.writeLock().unlock();
        }
    }

    private static native void nIndirectShaderHandle(long device, long indirect, long out);

    public ResidencySet createResidencySet() {
        lock.readLock().lock();
        try {
            return ResidencySet.of(nCreateResidencySet(handle()));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateResidencySet(long device);

    public void destroyResidencySet(ResidencySet set) {
        lock.readLock().lock();
        try {
            nDestroyResidencySet(handle(), ResidencySet.raw(set));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyResidencySet(long device, long set);

    public void addResidency(ResidencySet set, Resource[] resources) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var resourcesSize = 4;
                for (var x : resources) {
                    resourcesSize += x.encodedSize();
                }
                var resourcesAddress = arena.alloc(resourcesSize);
                var resourcesAt = resourcesAddress;
                memPutInt(resourcesAt, resources.length);
                resourcesAt += 4;
                for (var x : resources) {
                    resourcesAt = x.encode(resourcesAt);
                }
                nAddResidency(handle(), ResidencySet.raw(set), resourcesAddress, resourcesSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nAddResidency(long device, long set, long resources, int resourcesLength);

    public void removeResidency(ResidencySet set, Resource[] resources) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var resourcesSize = 4;
                for (var x : resources) {
                    resourcesSize += x.encodedSize();
                }
                var resourcesAddress = arena.alloc(resourcesSize);
                var resourcesAt = resourcesAddress;
                memPutInt(resourcesAt, resources.length);
                resourcesAt += 4;
                for (var x : resources) {
                    resourcesAt = x.encode(resourcesAt);
                }
                nRemoveResidency(handle(), ResidencySet.raw(set), resourcesAddress, resourcesSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nRemoveResidency(long device, long set, long resources, int resourcesLength);

    public void commitResidency(ResidencySet set) {
        lock.readLock().lock();
        try {
            nCommitResidency(handle(), ResidencySet.raw(set));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nCommitResidency(long device, long set);

    public Timeline createTimeline() {
        lock.readLock().lock();
        try {
            return new Timeline(nCreateTimeline(handle()));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateTimeline(long device);

    public void destroyTimeline(Timeline timeline) {
        lock.readLock().lock();
        try {
            nDestroyTimeline(handle(), timeline.take());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyTimeline(long device, long timeline);

    public CommandPool createCommandPool(int queue) {
        lock.readLock().lock();
        try {
            return CommandPool.of(nCreateCommandPool(handle(), queue));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateCommandPool(long device, int queue);

    public void resetCommandPool(CommandPool pool) {
        lock.writeLock().lock();
        try {
            idle("resetCommandPool");
            nResetCommandPool(handle(), CommandPool.raw(pool));
        } finally {
            lock.writeLock().unlock();
        }
    }

    private static native void nResetCommandPool(long device, long pool);

    public void destroyCommandPool(CommandPool pool) {
        lock.readLock().lock();
        try {
            nDestroyCommandPool(handle(), CommandPool.raw(pool));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroyCommandPool(long device, long pool);

    public CommandBuffer beginCommands(CommandPool pool) {
        lock.readLock().lock();
        try {
            return CommandBuffer.adopt(this, nBeginCommands(handle(), CommandPool.raw(pool)));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nBeginCommands(long device, long pool);

    public void submit(CommandBuffer[] commands, TimelinePoint[] waits, TimelinePoint[] signals) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                for (var c : commands) {
                    c.handle();
                }
                var commandsAddress = arena.alloc(commands.length * 8L);
                for (var i = 0; i < commands.length; i++) {
                    memPutLong(commandsAddress + i * 8, commands[i].take());
                }
                var waitsAddress = arena.alloc(waits.length * 16L);
                for (var i = 0; i < waits.length; i++) {
                    memPutLong(waitsAddress + i * 16, waits[i].timeline().handle(arena));
                    memPutLong(waitsAddress + i * 16 + 8, waits[i].value());
                }
                var signalsAddress = arena.alloc(signals.length * 16L);
                for (var i = 0; i < signals.length; i++) {
                    memPutLong(signalsAddress + i * 16, signals[i].timeline().handle(arena));
                    memPutLong(signalsAddress + i * 16 + 8, signals[i].value());
                }
                nSubmit(handle(), commandsAddress, commands.length, waitsAddress, waits.length, signalsAddress, signals.length);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nSubmit(long device, long commands, int commandsLength, long waits, int waitsLength, long signals, int signalsLength);
}
