package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class Device implements AutoCloseable {
    protected long handle;
    int recording;

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
        if (recording > 0) {
            throw new IllegalStateException(call + " changes the device while " + recording
                    + " command buffers or imported passes are recording; submit, finish or close them first");
        }
    }

    @Override
    public synchronized void close() {
        idle("close");
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
            nCaps(handle(), out);
            return Caps.decode(out);
        }
    }

    private static native void nCaps(long device, long out);

    public synchronized String deviceName() {
        return nDeviceName(handle());
    }

    private static native String nDeviceName(long device);

    public synchronized Buffer createBuffer(long size, int memory) {
        idle("createBuffer");
        return Buffer.of(nCreateBuffer(handle(), size, memory));
    }

    private static native long nCreateBuffer(long device, long size, int memory);

    public synchronized void destroyBuffer(Buffer buffer) {
        idle("destroyBuffer");
        nDestroyBuffer(handle(), Buffer.raw(buffer));
    }

    private static native void nDestroyBuffer(long device, long buffer);

    public synchronized long mapBuffer(Buffer buffer) {
        return nMapBuffer(handle(), Buffer.raw(buffer));
    }

    private static native long nMapBuffer(long device, long buffer);

    public synchronized void unmapBuffer(Buffer buffer) {
        nUnmapBuffer(handle(), Buffer.raw(buffer));
    }

    private static native void nUnmapBuffer(long device, long buffer);

    public synchronized long bufferAddress(Buffer buffer) {
        return nBufferAddress(handle(), Buffer.raw(buffer));
    }

    private static native long nBufferAddress(long device, long buffer);

    public synchronized MemoryBudget memoryBudget() {
        try (var arena = new Arena()) {
            var out = arena.alloc(MemoryBudget.SIZE);
            nMemoryBudget(handle(), out);
            return MemoryBudget.decode(out);
        }
    }

    private static native void nMemoryBudget(long device, long out);

    public synchronized ComputePipeline createComputePipeline(ComputePipelineInfo info) {
        idle("createComputePipeline");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return ComputePipeline.of(nCreateComputePipeline(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateComputePipeline(long device, long info, int infoLength);

    public synchronized void destroyComputePipeline(ComputePipeline pipeline) {
        idle("destroyComputePipeline");
        nDestroyComputePipeline(handle(), ComputePipeline.raw(pipeline));
    }

    private static native void nDestroyComputePipeline(long device, long pipeline);

    public synchronized RenderPipeline createRenderPipeline(RenderPipelineInfo info) {
        idle("createRenderPipeline");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return RenderPipeline.of(nCreateRenderPipeline(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateRenderPipeline(long device, long info, int infoLength);

    public synchronized RenderPipeline createMeshPipeline(MeshPipelineInfo info) {
        idle("createMeshPipeline");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return RenderPipeline.of(nCreateMeshPipeline(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateMeshPipeline(long device, long info, int infoLength);

    public synchronized void destroyRenderPipeline(RenderPipeline pipeline) {
        idle("destroyRenderPipeline");
        nDestroyRenderPipeline(handle(), RenderPipeline.raw(pipeline));
    }

    private static native void nDestroyRenderPipeline(long device, long pipeline);

    public synchronized TilePipeline createTilePipeline(TilePipelineInfo info) {
        idle("createTilePipeline");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return TilePipeline.of(nCreateTilePipeline(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateTilePipeline(long device, long info, int infoLength);

    public synchronized void destroyTilePipeline(TilePipeline pipeline) {
        idle("destroyTilePipeline");
        nDestroyTilePipeline(handle(), TilePipeline.raw(pipeline));
    }

    private static native void nDestroyTilePipeline(long device, long pipeline);

    public synchronized DepthState createDepthState(DepthStateInfo info) {
        idle("createDepthState");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return DepthState.of(nCreateDepthState(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateDepthState(long device, long info, int infoLength);

    public synchronized void destroyDepthState(DepthState state) {
        idle("destroyDepthState");
        nDestroyDepthState(handle(), DepthState.raw(state));
    }

    private static native void nDestroyDepthState(long device, long state);

    public synchronized Texture createTexture(TextureInfo info) {
        idle("createTexture");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return Texture.of(nCreateTexture(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateTexture(long device, long info, int infoLength);

    public synchronized void writeTexture(HostTextureCopy copy, long data, long dataSize) {
        idle("writeTexture");
        try (var arena = new Arena()) {
            var copyAddress = arena.alloc(copy.encodedSize());
            copy.encode(copyAddress);
            nWriteTexture(handle(), copyAddress, copy.encodedSize(), data, dataSize);
        }
    }

    private static native void nWriteTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized void readTexture(HostTextureCopy copy, long data, long dataSize) {
        idle("readTexture");
        try (var arena = new Arena()) {
            var copyAddress = arena.alloc(copy.encodedSize());
            copy.encode(copyAddress);
            nReadTexture(handle(), copyAddress, copy.encodedSize(), data, dataSize);
        }
    }

    private static native void nReadTexture(long device, long copy, int copyLength, long data, long dataLength);

    public synchronized int formatUsage(int format) {
        return nFormatUsage(handle(), format);
    }

    private static native int nFormatUsage(long device, int format);

    public synchronized int formatSamples(int format, int usage) {
        return nFormatSamples(handle(), format, usage);
    }

    private static native int nFormatSamples(long device, int format, int usage);

    public synchronized Texture createTextureView(Texture texture, TextureViewInfo info) {
        idle("createTextureView");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return Texture.of(nCreateTextureView(handle(), Texture.raw(texture), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateTextureView(long device, long texture, long info, int infoLength);

    public synchronized void destroyTexture(Texture texture) {
        idle("destroyTexture");
        nDestroyTexture(handle(), Texture.raw(texture));
    }

    private static native void nDestroyTexture(long device, long texture);

    public synchronized long bindlessStorageTexture(Texture texture) {
        return nBindlessStorageTexture(handle(), Texture.raw(texture));
    }

    private static native long nBindlessStorageTexture(long device, long texture);

    public synchronized Sampler createSampler(SamplerInfo info) {
        idle("createSampler");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return Sampler.of(nCreateSampler(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateSampler(long device, long info, int infoLength);

    public synchronized void destroySampler(Sampler sampler) {
        idle("destroySampler");
        nDestroySampler(handle(), Sampler.raw(sampler));
    }

    private static native void nDestroySampler(long device, long sampler);

    public synchronized QueryPool createQueryPool(int kind, int count) {
        idle("createQueryPool");
        return QueryPool.of(nCreateQueryPool(handle(), kind, count));
    }

    private static native long nCreateQueryPool(long device, int kind, int count);

    public synchronized void destroyQueryPool(QueryPool pool) {
        idle("destroyQueryPool");
        nDestroyQueryPool(handle(), QueryPool.raw(pool));
    }

    private static native void nDestroyQueryPool(long device, long pool);

    public synchronized IndirectBuffer createIndirectBuffer(IndirectBufferInfo info) {
        idle("createIndirectBuffer");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return IndirectBuffer.of(nCreateIndirectBuffer(handle(), infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateIndirectBuffer(long device, long info, int infoLength);

    public synchronized void destroyIndirectBuffer(IndirectBuffer indirect) {
        idle("destroyIndirectBuffer");
        nDestroyIndirectBuffer(handle(), IndirectBuffer.raw(indirect));
    }

    private static native void nDestroyIndirectBuffer(long device, long indirect);

    public synchronized int[] indirectShaderHandle(IndirectBuffer indirect) {
        idle("indirectShaderHandle");
        try (var arena = new Arena()) {
            var out = arena.alloc(32);
            nIndirectShaderHandle(handle(), IndirectBuffer.raw(indirect), out);
            return new int[] {memGetInt(out + 0), memGetInt(out + 4), memGetInt(out + 8), memGetInt(out + 12), memGetInt(out + 16), memGetInt(out + 20), memGetInt(out + 24), memGetInt(out + 28)};
        }
    }

    private static native void nIndirectShaderHandle(long device, long indirect, long out);

    public synchronized ResidencySet createResidencySet() {
        idle("createResidencySet");
        return ResidencySet.of(nCreateResidencySet(handle()));
    }

    private static native long nCreateResidencySet(long device);

    public synchronized void destroyResidencySet(ResidencySet set) {
        idle("destroyResidencySet");
        nDestroyResidencySet(handle(), ResidencySet.raw(set));
    }

    private static native void nDestroyResidencySet(long device, long set);

    public synchronized void addResidency(ResidencySet set, Buffer[] buffers, Texture[] textures, QueryPool[] queries, IndirectBuffer[] indirects) {
        idle("addResidency");
        try (var arena = new Arena()) {
            var buffersSize = 4;
            for (var x : buffers) {
                buffersSize += 8;
            }
            var buffersAddress = arena.alloc(buffersSize);
            var buffersAt = buffersAddress;
            memPutInt(buffersAt, buffers.length);
            buffersAt += 4;
            for (var x : buffers) {
                memPutLong(buffersAt, Buffer.raw(x));
                buffersAt += 8;
            }
            var texturesSize = 4;
            for (var x : textures) {
                texturesSize += 8;
            }
            var texturesAddress = arena.alloc(texturesSize);
            var texturesAt = texturesAddress;
            memPutInt(texturesAt, textures.length);
            texturesAt += 4;
            for (var x : textures) {
                memPutLong(texturesAt, Texture.raw(x));
                texturesAt += 8;
            }
            var queriesSize = 4;
            for (var x : queries) {
                queriesSize += 8;
            }
            var queriesAddress = arena.alloc(queriesSize);
            var queriesAt = queriesAddress;
            memPutInt(queriesAt, queries.length);
            queriesAt += 4;
            for (var x : queries) {
                memPutLong(queriesAt, QueryPool.raw(x));
                queriesAt += 8;
            }
            var indirectsSize = 4;
            for (var x : indirects) {
                indirectsSize += 8;
            }
            var indirectsAddress = arena.alloc(indirectsSize);
            var indirectsAt = indirectsAddress;
            memPutInt(indirectsAt, indirects.length);
            indirectsAt += 4;
            for (var x : indirects) {
                memPutLong(indirectsAt, IndirectBuffer.raw(x));
                indirectsAt += 8;
            }
            nAddResidency(handle(), ResidencySet.raw(set), buffersAddress, buffersSize, texturesAddress, texturesSize, queriesAddress, queriesSize, indirectsAddress, indirectsSize);
        }
    }

    private static native void nAddResidency(long device, long set, long buffers, int buffersLength, long textures, int texturesLength, long queries, int queriesLength, long indirects, int indirectsLength);

    public synchronized void removeResidency(ResidencySet set, Buffer[] buffers, Texture[] textures, QueryPool[] queries, IndirectBuffer[] indirects) {
        idle("removeResidency");
        try (var arena = new Arena()) {
            var buffersSize = 4;
            for (var x : buffers) {
                buffersSize += 8;
            }
            var buffersAddress = arena.alloc(buffersSize);
            var buffersAt = buffersAddress;
            memPutInt(buffersAt, buffers.length);
            buffersAt += 4;
            for (var x : buffers) {
                memPutLong(buffersAt, Buffer.raw(x));
                buffersAt += 8;
            }
            var texturesSize = 4;
            for (var x : textures) {
                texturesSize += 8;
            }
            var texturesAddress = arena.alloc(texturesSize);
            var texturesAt = texturesAddress;
            memPutInt(texturesAt, textures.length);
            texturesAt += 4;
            for (var x : textures) {
                memPutLong(texturesAt, Texture.raw(x));
                texturesAt += 8;
            }
            var queriesSize = 4;
            for (var x : queries) {
                queriesSize += 8;
            }
            var queriesAddress = arena.alloc(queriesSize);
            var queriesAt = queriesAddress;
            memPutInt(queriesAt, queries.length);
            queriesAt += 4;
            for (var x : queries) {
                memPutLong(queriesAt, QueryPool.raw(x));
                queriesAt += 8;
            }
            var indirectsSize = 4;
            for (var x : indirects) {
                indirectsSize += 8;
            }
            var indirectsAddress = arena.alloc(indirectsSize);
            var indirectsAt = indirectsAddress;
            memPutInt(indirectsAt, indirects.length);
            indirectsAt += 4;
            for (var x : indirects) {
                memPutLong(indirectsAt, IndirectBuffer.raw(x));
                indirectsAt += 8;
            }
            nRemoveResidency(handle(), ResidencySet.raw(set), buffersAddress, buffersSize, texturesAddress, texturesSize, queriesAddress, queriesSize, indirectsAddress, indirectsSize);
        }
    }

    private static native void nRemoveResidency(long device, long set, long buffers, int buffersLength, long textures, int texturesLength, long queries, int queriesLength, long indirects, int indirectsLength);

    public synchronized void commitResidency(ResidencySet set) {
        idle("commitResidency");
        nCommitResidency(handle(), ResidencySet.raw(set));
    }

    private static native void nCommitResidency(long device, long set);

    public synchronized Timeline createTimeline() {
        idle("createTimeline");
        return new Timeline(nCreateTimeline(handle()));
    }

    private static native long nCreateTimeline(long device);

    public synchronized void destroyTimeline(Timeline timeline) {
        idle("destroyTimeline");
        nDestroyTimeline(handle(), timeline.take());
    }

    private static native void nDestroyTimeline(long device, long timeline);

    public synchronized CommandPool createCommandPool(int queue) {
        idle("createCommandPool");
        return CommandPool.of(nCreateCommandPool(handle(), queue));
    }

    private static native long nCreateCommandPool(long device, int queue);

    public synchronized void resetCommandPool(CommandPool pool) {
        idle("resetCommandPool");
        nResetCommandPool(handle(), CommandPool.raw(pool));
    }

    private static native void nResetCommandPool(long device, long pool);

    public synchronized void destroyCommandPool(CommandPool pool) {
        idle("destroyCommandPool");
        nDestroyCommandPool(handle(), CommandPool.raw(pool));
    }

    private static native void nDestroyCommandPool(long device, long pool);

    public synchronized CommandBuffer beginCommands(CommandPool pool) {
        return CommandBuffer.adopt(this, nBeginCommands(handle(), CommandPool.raw(pool)));
    }

    private static native long nBeginCommands(long device, long pool);

    public synchronized void submit(CommandBuffer commands, TimelinePoint[] waits, TimelinePoint[] signals) {
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
            nSubmit(handle(), commands.take(), waitsAddress, waits.length, signalsAddress, signals.length);
        }
    }

    private static native void nSubmit(long device, long commands, long waits, int waitsLength, long signals, int signalsLength);
}
