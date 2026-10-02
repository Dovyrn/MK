package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.lang.ref.Reference;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class VkDevice {

    public static long address(Device device, int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nAddress(device.handle, buffer);
        }
    }

    private static native long nAddress(long device, int buffer);

    public static int createHeap(Device device, long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateHeap(device.handle, size, memory);
        }
    }

    private static native int nCreateHeap(long device, long size, int memory);

    public static void destroyHeap(Device device, int heap) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyHeap(device.handle, heap);
        }
    }

    private static native void nDestroyHeap(long device, int heap);

    public static long[] bufferSize(Device device, long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nBufferSize(device.handle, size, memory, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nBufferSize(long device, long size, int memory, long out);

    public static long[] textureSize(Device device, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 256);
            nTextureSize(device.handle, infoAddress, info.size(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native void nTextureSize(long device, long info, int infoLength, long out);

    public static int createPlacedBuffer(Device device, int heap, long offset, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreatePlacedBuffer(device.handle, heap, offset, size);
        }
    }

    private static native int nCreatePlacedBuffer(long device, int heap, long offset, long size);

    public static int createPlacedTexture(Device device, int heap, long offset, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreatePlacedTexture(device.handle, heap, offset, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreatePlacedTexture(long device, int heap, long offset, long info, int infoLength);

    public static long textureHandle(Device device, int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nTextureHandle(device.handle, texture);
        }
    }

    private static native long nTextureHandle(long device, int texture);

    public static long imageHandle(Device device, int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nImageHandle(device.handle, texture);
        }
    }

    private static native long nImageHandle(long device, int texture);

    public static long samplerHandle(Device device, int sampler) {
        try (var stack = MemoryStack.stackPush()) {
            return nSamplerHandle(device.handle, sampler);
        }
    }

    private static native long nSamplerHandle(long device, int sampler);

    public static int createAccel(Device device, AccelInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateAccel(device.handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateAccel(long device, long info, int infoLength);

    public static void destroyAccel(Device device, int accel) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyAccel(device.handle, accel);
        }
    }

    private static native void nDestroyAccel(long device, int accel);

    public static long[] accelScratch(Device device, int accel) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nAccelScratch(device.handle, accel, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nAccelScratch(long device, int accel, long out);

    public static long accelHandle(Device device, int accel) {
        try (var stack = MemoryStack.stackPush()) {
            return nAccelHandle(device.handle, accel);
        }
    }

    private static native long nAccelHandle(long device, int accel);

    public static void writeInstance(Device device, ByteBuffer out, int accel, float[] transform, int id, int mask, int flags) {
        try (var stack = MemoryStack.stackPush()) {
            nWriteInstance(device.handle, memAddress(out), out.remaining(), accel, memAddress(stack.floats(transform)), id, mask, flags);
        } finally {
            Reference.reachabilityFence(out);
        }
    }

    private static native void nWriteInstance(long device, long out, long outLength, int accel, long transform, int id, int mask, int flags);

    public static Device adoptRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var stack = MemoryStack.stackPush()) {
            return new Device(nAdoptRaw(proc, instance, physical, device, memAddress(stack.ints(queues)), queues.length / 2, features));
        }
    }

    private static native long nAdoptRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public static int importBuffer(Device device, long raw) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportBuffer(device.handle, raw);
        }
    }

    private static native int nImportBuffer(long device, long raw);

    public static int importTexture(Device device, long image, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nImportTexture(device.handle, image, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nImportTexture(long device, long image, long info, int infoLength);

    public static int importEvent(Device device, long raw) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportEvent(device.handle, raw);
        }
    }

    private static native int nImportEvent(long device, long raw);

    public static long rawBuffer(Device device, int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawBuffer(device.handle, buffer);
        }
    }

    private static native long nRawBuffer(long device, int buffer);

    public static long[] rawTexture(Device device, int texture) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nRawTexture(device.handle, texture, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nRawTexture(long device, int texture, long out);

    public static long rawEvent(Device device, int event) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawEvent(device.handle, event);
        }
    }

    private static native long nRawEvent(long device, int event);

    public static long rawQueue(Device device, int queue) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawQueue(device.handle, queue);
        }
    }

    private static native long nRawQueue(long device, int queue);

    public static int createCompacted(Device device, int accel, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateCompacted(device.handle, accel, size);
        }
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public static int createHostBuffer(Device device, long pointer, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateHostBuffer(device.handle, pointer, size);
        }
    }

    private static native int nCreateHostBuffer(long device, long pointer, long size);

    public static long sparsePage(Device device) {
        try (var stack = MemoryStack.stackPush()) {
            return nSparsePage(device.handle);
        }
    }

    private static native long nSparsePage(long device);

    public static int[] sparseTile(Device device, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 256);
            nSparseTile(device.handle, infoAddress, info.size(), out);
            return new int[] {memGetInt(out), memGetInt(out + 4), memGetInt(out + 8)};
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native void nSparseTile(long device, long info, int infoLength, long out);

    public static int createSparseBuffer(Device device, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateSparseBuffer(device.handle, size);
        }
    }

    private static native int nCreateSparseBuffer(long device, long size);

    public static int createSparseTexture(Device device, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateSparseTexture(device.handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateSparseTexture(long device, long info, int infoLength);

    public static void mapSparseBuffer(Device device, int queue, int buffer, int heap, SparseRange[] ranges, long[] waits, long[] signals) {
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
            nMapSparseBuffer(device.handle, queue, buffer, heap, rangesAddress, rangesSize, waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nMapSparseBuffer(long device, int queue, int buffer, int heap, long ranges, int rangesLength, long waits, int waitsLength, long signals, int signalsLength);

    public static void mapSparseTexture(Device device, int queue, int texture, int heap, SparseRegion[] regions, long[] waits, long[] signals) {
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
            nMapSparseTexture(device.handle, queue, texture, heap, regionsAddress, regionsSize, waitsAddress, waits.length / 2, signalsAddress, signals.length / 2);
        }
    }

    private static native void nMapSparseTexture(long device, int queue, int texture, int heap, long regions, int regionsLength, long waits, int waitsLength, long signals, int signalsLength);

    public static int createCommandsLayout(Device device, LayoutInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateCommandsLayout(device.handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateCommandsLayout(long device, long info, int infoLength);

    public static void destroyCommandsLayout(Device device, int layout) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyCommandsLayout(device.handle, layout);
        }
    }

    private static native void nDestroyCommandsLayout(long device, int layout);

    public static long generatedSize(Device device, int layout, int pipeline, int sequences, int draws) {
        try (var stack = MemoryStack.stackPush()) {
            return nGeneratedSize(device.handle, layout, pipeline, sequences, draws);
        }
    }

    private static native long nGeneratedSize(long device, int layout, int pipeline, int sequences, int draws);

    public static int createRayPipeline(Device device, RayInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateRayPipeline(device.handle, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateRayPipeline(long device, long info, int infoLength);

    public static void destroyRayPipeline(Device device, int pipeline) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyRayPipeline(device.handle, pipeline);
        }
    }

    private static native void nDestroyRayPipeline(long device, int pipeline);

    public static byte[] rayHandles(Device device, int pipeline) {
        try (var stack = MemoryStack.stackPush()) {
            return nRayHandles(device.handle, pipeline);
        }
    }

    private static native byte[] nRayHandles(long device, int pipeline);

    public static int createSharedHeap(Device device, long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateSharedHeap(device.handle, size, memory);
        }
    }

    private static native int nCreateSharedHeap(long device, long size, int memory);

    public static int exportHeap(Device device, int heap) {
        try (var stack = MemoryStack.stackPush()) {
            return nExportHeap(device.handle, heap);
        }
    }

    private static native int nExportHeap(long device, int heap);

    public static int importHeap(Device device, int fd, long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportHeap(device.handle, fd, size, memory);
        }
    }

    private static native int nImportHeap(long device, int fd, long size, int memory);
}
