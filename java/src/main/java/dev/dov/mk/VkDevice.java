package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.lang.ref.Reference;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class VkDevice extends Device {
    public VkDevice(long handle) {
        super(handle);
    }

    public long address(int buffer) {
        return nAddress(handle, buffer);
    }

    private static native long nAddress(long device, int buffer);

    public int createHeap(long size, int memory) {
        return nCreateHeap(handle, size, memory);
    }

    private static native int nCreateHeap(long device, long size, int memory);

    public void destroyHeap(int heap) {
        nDestroyHeap(handle, heap);
    }

    private static native void nDestroyHeap(long device, int heap);

    public long[] bufferSize(long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nBufferSize(handle, size, memory, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nBufferSize(long device, long size, int memory, long out);

    public long[] textureSize(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 16);
            nTextureSize(handle, infoAddress, info.size(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native void nTextureSize(long device, long info, int infoLength, long out);

    public int createPlacedBuffer(int heap, long offset, long size) {
        return nCreatePlacedBuffer(handle, heap, offset, size);
    }

    private static native int nCreatePlacedBuffer(long device, int heap, long offset, long size);

    public int createPlacedTexture(int heap, long offset, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreatePlacedTexture(handle, heap, offset, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreatePlacedTexture(long device, int heap, long offset, long info, int infoLength);

    public long textureHandle(int texture) {
        return nTextureHandle(handle, texture);
    }

    private static native long nTextureHandle(long device, int texture);

    public long imageHandle(int texture) {
        return nImageHandle(handle, texture);
    }

    private static native long nImageHandle(long device, int texture);

    public long samplerHandle(int sampler) {
        return nSamplerHandle(handle, sampler);
    }

    private static native long nSamplerHandle(long device, int sampler);

    public int createAccel(AccelInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateAccel(handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateAccel(long device, long info, int infoLength);

    public void destroyAccel(int accel) {
        nDestroyAccel(handle, accel);
    }

    private static native void nDestroyAccel(long device, int accel);

    public long[] accelScratch(int accel) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nAccelScratch(handle, accel, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nAccelScratch(long device, int accel, long out);

    public long accelHandle(int accel) {
        return nAccelHandle(handle, accel);
    }

    private static native long nAccelHandle(long device, int accel);

    public void writeInstance(ByteBuffer out, int accel, float[] transform, int id, int mask, int flags) {
        try (var stack = MemoryStack.stackPush()) {
            if (!out.isDirect()) {
                throw new IllegalArgumentException("out must be a direct ByteBuffer");
            }
            nWriteInstance(handle, memAddress(out), out.remaining(), accel, memAddress(stack.floats(transform)), id, mask, flags);
        } finally {
            Reference.reachabilityFence(out);
        }
    }

    private static native void nWriteInstance(long device, long out, long outLength, int accel, long transform, int id, int mask, int flags);

    public static VkDevice adoptRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var stack = MemoryStack.stackPush()) {
            return new VkDevice(nAdoptRaw(proc, instance, physical, device, memAddress(stack.ints(queues)), queues.length / 2, features));
        }
    }

    private static native long nAdoptRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public int importBuffer(long raw) {
        return nImportBuffer(handle, raw);
    }

    private static native int nImportBuffer(long device, long raw);

    public int importTexture(long image, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nImportTexture(handle, image, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nImportTexture(long device, long image, long info, int infoLength);

    public int importEvent(long raw) {
        return nImportEvent(handle, raw);
    }

    private static native int nImportEvent(long device, long raw);

    public long rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native long nRawBuffer(long device, int buffer);

    public long[] rawTexture(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nRawTexture(handle, texture, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nRawTexture(long device, int texture, long out);

    public long rawEvent(int event) {
        return nRawEvent(handle, event);
    }

    private static native long nRawEvent(long device, int event);

    public long rawQueue(int queue) {
        return nRawQueue(handle, queue);
    }

    private static native long nRawQueue(long device, int queue);

    public int createCompacted(int accel, long size) {
        return nCreateCompacted(handle, accel, size);
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public int createHostBuffer(long pointer, long size) {
        return nCreateHostBuffer(handle, pointer, size);
    }

    private static native int nCreateHostBuffer(long device, long pointer, long size);

    public long sparsePage() {
        return nSparsePage(handle);
    }

    private static native long nSparsePage(long device);

    public int[] sparseTile(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 12);
            nSparseTile(handle, infoAddress, info.size(), out);
            return new int[] {memGetInt(out), memGetInt(out + 4), memGetInt(out + 8)};
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native void nSparseTile(long device, long info, int infoLength, long out);

    public int createSparseBuffer(long size) {
        return nCreateSparseBuffer(handle, size);
    }

    private static native int nCreateSparseBuffer(long device, long size);

    public int createSparseTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateSparseTexture(handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
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

    public int createCommandsLayout(LayoutInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateCommandsLayout(handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateCommandsLayout(long device, long info, int infoLength);

    public void destroyCommandsLayout(int layout) {
        nDestroyCommandsLayout(handle, layout);
    }

    private static native void nDestroyCommandsLayout(long device, int layout);

    public long generatedSize(int layout, int pipeline, int sequences, int draws) {
        return nGeneratedSize(handle, layout, pipeline, sequences, draws);
    }

    private static native long nGeneratedSize(long device, int layout, int pipeline, int sequences, int draws);

    public int createRayPipeline(RayInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateRayPipeline(handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateRayPipeline(long device, long info, int infoLength);

    public void destroyRayPipeline(int pipeline) {
        nDestroyRayPipeline(handle, pipeline);
    }

    private static native void nDestroyRayPipeline(long device, int pipeline);

    public byte[] rayHandles(int pipeline) {
        return nRayHandles(handle, pipeline);
    }

    private static native byte[] nRayHandles(long device, int pipeline);

    public int createSharedHeap(long size, int memory) {
        return nCreateSharedHeap(handle, size, memory);
    }

    private static native int nCreateSharedHeap(long device, long size, int memory);

    public int exportHeap(int heap) {
        return nExportHeap(handle, heap);
    }

    private static native int nExportHeap(long device, int heap);

    public int importHeap(int fd, long size, int memory) {
        return nImportHeap(handle, fd, size, memory);
    }

    private static native int nImportHeap(long device, int fd, long size, int memory);
}
