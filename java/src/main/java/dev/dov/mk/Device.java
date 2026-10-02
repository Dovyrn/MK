package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class Device {
    public final long handle;

    public Device(long handle) {
        this.handle = handle;
    }

    public void free() {
        nFree(handle);
    }

    private static native void nFree(long device);

    public Caps caps() {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nCaps(handle, out);
            return Caps.read(out);
        }
    }

    private static native void nCaps(long device, long out);

    public String name() {
        try (var stack = MemoryStack.stackPush()) {
            return nName(handle);
        }
    }

    private static native String nName(long device);

    public int createBuffer(long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateBuffer(handle, size, memory);
        }
    }

    private static native int nCreateBuffer(long device, long size, int memory);

    public void destroyBuffer(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyBuffer(handle, buffer);
        }
    }

    private static native void nDestroyBuffer(long device, int buffer);

    public long map(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nMap(handle, buffer);
        }
    }

    private static native long nMap(long device, int buffer);

    public long bufferHandle(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nBufferHandle(handle, buffer);
        }
    }

    private static native long nBufferHandle(long device, int buffer);

    public void unmap(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            nUnmap(handle, buffer);
        }
    }

    private static native void nUnmap(long device, int buffer);

    public long address(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nAddress(handle, buffer);
        }
    }

    private static native long nAddress(long device, int buffer);

    public int createHeap(long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateHeap(handle, size, memory);
        }
    }

    private static native int nCreateHeap(long device, long size, int memory);

    public void destroyHeap(int heap) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyHeap(handle, heap);
        }
    }

    private static native void nDestroyHeap(long device, int heap);

    public long[] bufferSize(long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nBufferSize(handle, size, memory, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nBufferSize(long device, long size, int memory, long out);

    public long[] textureSize(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 256);
            nTextureSize(handle, infoAddress, info.size(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nTextureSize(long device, long info, int infoLength, long out);

    public int createPlacedBuffer(int heap, long offset, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreatePlacedBuffer(handle, heap, offset, size);
        }
    }

    private static native int nCreatePlacedBuffer(long device, int heap, long offset, long size);

    public int createPlacedTexture(int heap, long offset, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreatePlacedTexture(handle, heap, offset, infoAddress, info.size());
        }
    }

    private static native int nCreatePlacedTexture(long device, int heap, long offset, long info, int infoLength);

    public int createPipeline(ComputeInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreatePipeline(handle, infoAddress, info.size());
        }
    }

    private static native int nCreatePipeline(long device, long info, int infoLength);

    public void destroyPipeline(int pipeline) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyPipeline(handle, pipeline);
        }
    }

    private static native void nDestroyPipeline(long device, int pipeline);

    public int createRenderPipeline(RenderInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateRenderPipeline(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateRenderPipeline(long device, long info, int infoLength);

    public int createMeshPipeline(MeshInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateMeshPipeline(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateMeshPipeline(long device, long info, int infoLength);

    public void destroyRenderPipeline(int pipeline) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyRenderPipeline(handle, pipeline);
        }
    }

    private static native void nDestroyRenderPipeline(long device, int pipeline);

    public int createDepthState(DepthInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateDepthState(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateDepthState(long device, long info, int infoLength);

    public void destroyDepthState(int state) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyDepthState(handle, state);
        }
    }

    private static native void nDestroyDepthState(long device, int state);

    public int createTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateTexture(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateTexture(long device, long info, int infoLength);

    public int formatUsage(int format) {
        try (var stack = MemoryStack.stackPush()) {
            return nFormatUsage(handle, format);
        }
    }

    private static native int nFormatUsage(long device, int format);

    public int createView(int texture, ViewInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateView(handle, texture, infoAddress, info.size());
        }
    }

    private static native int nCreateView(long device, int texture, long info, int infoLength);

    public void destroyTexture(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyTexture(handle, texture);
        }
    }

    private static native void nDestroyTexture(long device, int texture);

    public long textureHandle(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nTextureHandle(handle, texture);
        }
    }

    private static native long nTextureHandle(long device, int texture);

    public long imageHandle(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nImageHandle(handle, texture);
        }
    }

    private static native long nImageHandle(long device, int texture);

    public int createSampler(SamplerInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateSampler(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateSampler(long device, long info, int infoLength);

    public void destroySampler(int sampler) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroySampler(handle, sampler);
        }
    }

    private static native void nDestroySampler(long device, int sampler);

    public long samplerHandle(int sampler) {
        try (var stack = MemoryStack.stackPush()) {
            return nSamplerHandle(handle, sampler);
        }
    }

    private static native long nSamplerHandle(long device, int sampler);

    public int createAccel(AccelInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateAccel(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateAccel(long device, long info, int infoLength);

    public void destroyAccel(int accel) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyAccel(handle, accel);
        }
    }

    private static native void nDestroyAccel(long device, int accel);

    public long[] accelScratch(int accel) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nAccelScratch(handle, accel, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nAccelScratch(long device, int accel, long out);

    public long accelHandle(int accel) {
        try (var stack = MemoryStack.stackPush()) {
            return nAccelHandle(handle, accel);
        }
    }

    private static native long nAccelHandle(long device, int accel);

    public void writeInstance(ByteBuffer out, int accel, float[] transform, int id, int mask, int flags) {
        try (var stack = MemoryStack.stackPush()) {
            nWriteInstance(handle, memAddress(out), out.remaining(), accel, memAddress(stack.floats(transform)), id, mask, flags);
        }
    }

    private static native void nWriteInstance(long device, long out, long outLength, int accel, long transform, int id, int mask, int flags);

    public int createQueryHeap(int kind, int count) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateQueryHeap(handle, kind, count);
        }
    }

    private static native int nCreateQueryHeap(long device, int kind, int count);

    public void destroyQueryHeap(int heap) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyQueryHeap(handle, heap);
        }
    }

    private static native void nDestroyQueryHeap(long device, int heap);

    public void recordCache() {
        try (var stack = MemoryStack.stackPush()) {
            nRecordCache(handle);
        }
    }

    private static native void nRecordCache(long device);

    public void saveCache(String path) {
        try (var stack = MemoryStack.stackPush()) {
            var pathBytes = stack.UTF8(path, false);
            nSaveCache(handle, memAddress(pathBytes), pathBytes.remaining());
        }
    }

    private static native void nSaveCache(long device, long path, int pathLength);

    public void loadCache(String path) {
        try (var stack = MemoryStack.stackPush()) {
            var pathBytes = stack.UTF8(path, false);
            nLoadCache(handle, memAddress(pathBytes), pathBytes.remaining());
        }
    }

    private static native void nLoadCache(long device, long path, int pathLength);

    public int createEvent() {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateEvent(handle);
        }
    }

    private static native int nCreateEvent(long device);

    public void destroyEvent(int event) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyEvent(handle, event);
        }
    }

    private static native void nDestroyEvent(long device, int event);

    public long eventValue(int event) {
        try (var stack = MemoryStack.stackPush()) {
            return nEventValue(handle, event);
        }
    }

    private static native long nEventValue(long device, int event);

    public boolean wait(int event, long value, long timeout) {
        try (var stack = MemoryStack.stackPush()) {
            return nWait(handle, event, value, timeout);
        }
    }

    private static native boolean nWait(long device, int event, long value, long timeout);

    public void submit(int queue, Commands commands, long[] waits, long[] signals) {
        try (var stack = MemoryStack.stackPush()) {
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            var signalsAddress = stack.nmalloc(8, signals.length * 8);
            for (var i = 0; i < signals.length / 2; i++) {
                memPutInt(signalsAddress + i * 16, (int) signals[i * 2]);
                memPutLong(signalsAddress + i * 16 + 8, signals[i * 2 + 1]);
            }
            nSubmit(handle, queue, commands.getAddress(), commands.getSize(), waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nSubmit(long device, int queue, long commands, long commandsLength, long waits, int waitsLength, long signals, int signalsLength);

    public int createCompacted(int accel, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateCompacted(handle, accel, size);
        }
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public int createHostBuffer(long pointer, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateHostBuffer(handle, pointer, size);
        }
    }

    private static native int nCreateHostBuffer(long device, long pointer, long size);

    public long sparsePage() {
        try (var stack = MemoryStack.stackPush()) {
            return nSparsePage(handle);
        }
    }

    private static native long nSparsePage(long device);

    public int[] sparseTile(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 256);
            nSparseTile(handle, infoAddress, info.size(), out);
            return new int[] {memGetInt(out), memGetInt(out + 4), memGetInt(out + 8)};
        }
    }

    private static native void nSparseTile(long device, long info, int infoLength, long out);

    public int createSparseBuffer(long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateSparseBuffer(handle, size);
        }
    }

    private static native int nCreateSparseBuffer(long device, long size);

    public int createSparseTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateSparseTexture(handle, infoAddress, info.size());
        }
    }

    private static native int nCreateSparseTexture(long device, long info, int infoLength);

    public void mapSparseBuffer(int queue, int buffer, int heap, SparseRange[] ranges, long[] waits, long[] signals) {
        try (var stack = MemoryStack.stackPush()) {
            var rangesSize = 4;
            for (var x : ranges) {
                rangesSize += x.size();
            }
            var rangesAddress = stack.nmalloc(8, rangesSize);
            memPutInt(rangesAddress, ranges.length);
            var rangesAt = rangesAddress + 4;
            for (var x : ranges) {
                rangesAt = x.write(rangesAt);
            }
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            var signalsAddress = stack.nmalloc(8, signals.length * 8);
            for (var i = 0; i < signals.length / 2; i++) {
                memPutInt(signalsAddress + i * 16, (int) signals[i * 2]);
                memPutLong(signalsAddress + i * 16 + 8, signals[i * 2 + 1]);
            }
            nMapSparseBuffer(handle, queue, buffer, heap, rangesAddress, rangesSize, waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nMapSparseBuffer(long device, int queue, int buffer, int heap, long ranges, int rangesLength, long waits, int waitsLength, long signals, int signalsLength);

    public void mapSparseTexture(int queue, int texture, int heap, SparseRegion[] regions, long[] waits, long[] signals) {
        try (var stack = MemoryStack.stackPush()) {
            var regionsSize = 4;
            for (var x : regions) {
                regionsSize += x.size();
            }
            var regionsAddress = stack.nmalloc(8, regionsSize);
            memPutInt(regionsAddress, regions.length);
            var regionsAt = regionsAddress + 4;
            for (var x : regions) {
                regionsAt = x.write(regionsAt);
            }
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            var signalsAddress = stack.nmalloc(8, signals.length * 8);
            for (var i = 0; i < signals.length / 2; i++) {
                memPutInt(signalsAddress + i * 16, (int) signals[i * 2]);
                memPutLong(signalsAddress + i * 16 + 8, signals[i * 2 + 1]);
            }
            nMapSparseTexture(handle, queue, texture, heap, regionsAddress, regionsSize, waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nMapSparseTexture(long device, int queue, int texture, int heap, long regions, int regionsLength, long waits, int waitsLength, long signals, int signalsLength);
}
