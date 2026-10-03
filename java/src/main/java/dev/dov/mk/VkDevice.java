package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class VkDevice extends Device {
    public VkDevice(long handle) {
        super(handle);
    }

    public synchronized Matrix matrix(int index) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, Matrix.SIZE);
            nMatrix(handle, index, out);
            return Matrix.decode(out);
        }
    }

    private static native void nMatrix(long device, int index, long out);

    public synchronized int createHeap(long size, int memory) {
        return nCreateHeap(handle, size, memory);
    }

    private static native int nCreateHeap(long device, long size, int memory);

    public synchronized long[] bufferSize(long size, int memory) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nBufferSize(handle, size, memory, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nBufferSize(long device, long size, int memory, long out);

    public synchronized long[] textureSize(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            var out = stack.nmalloc(8, 16);
            try {
                nTextureSize(handle, infoAddress, info.encodedSize(), out);
                return new long[] {memGetLong(out), memGetLong(out + 8)};
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native void nTextureSize(long device, long info, int infoLength, long out);

    public synchronized long textureHandle(int texture) {
        return nTextureHandle(handle, texture);
    }

    private static native long nTextureHandle(long device, int texture);

    public synchronized long samplerHandle(int sampler) {
        return nSamplerHandle(handle, sampler);
    }

    private static native long nSamplerHandle(long device, int sampler);

    public synchronized int createAccel(AccelInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateAccel(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateAccel(long device, long info, int infoLength);

    public synchronized void destroyAccel(int accel) {
        nDestroyAccel(handle, accel);
    }

    private static native void nDestroyAccel(long device, int accel);

    public synchronized long[] accelScratch(int accel) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nAccelScratch(handle, accel, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nAccelScratch(long device, int accel, long out);

    public synchronized long accelHandle(int accel) {
        return nAccelHandle(handle, accel);
    }

    private static native long nAccelHandle(long device, int accel);

    public synchronized void writeInstance(long out, long outSize, int accel, float[] transform, int id, int mask, int flags) {
        try (var stack = MemoryStack.stackPush()) {
            nWriteInstance(handle, out, outSize, accel, memAddress(stack.floats(transform)), id, mask, flags);
        }
    }

    private static native void nWriteInstance(long device, long out, long outLength, int accel, long transform, int id, int mask, int flags);

    public static VkDevice adoptRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var stack = MemoryStack.stackPush()) {
            return new VkDevice(nAdoptRaw(proc, instance, physical, device, memAddress(stack.ints(queues)), queues.length / 2, features));
        }
    }

    private static native long nAdoptRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public synchronized int importBuffer(long raw, long size) {
        return nImportBuffer(handle, raw, size);
    }

    private static native int nImportBuffer(long device, long raw, long size);

    public synchronized int importTexture(long image, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nImportTexture(handle, image, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nImportTexture(long device, long image, long info, int infoLength);

    public synchronized int importEvent(long raw) {
        return nImportEvent(handle, raw);
    }

    private static native int nImportEvent(long device, long raw);

    public synchronized long rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native long nRawBuffer(long device, int buffer);

    public synchronized long[] rawTexture(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 16);
            nRawTexture(handle, texture, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nRawTexture(long device, int texture, long out);

    public synchronized long rawEvent(int event) {
        return nRawEvent(handle, event);
    }

    private static native long nRawEvent(long device, int event);

    public synchronized long rawQueue(int queue) {
        return nRawQueue(handle, queue);
    }

    private static native long nRawQueue(long device, int queue);

    public synchronized int createCompacted(int accel, long size) {
        return nCreateCompacted(handle, accel, size);
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public synchronized int createCommandsLayout(LayoutInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateCommandsLayout(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateCommandsLayout(long device, long info, int infoLength);

    public synchronized void destroyCommandsLayout(int layout) {
        nDestroyCommandsLayout(handle, layout);
    }

    private static native void nDestroyCommandsLayout(long device, int layout);

    public synchronized long generatedSize(int layout, int pipeline, int sequences, int draws) {
        return nGeneratedSize(handle, layout, pipeline, sequences, draws);
    }

    private static native long nGeneratedSize(long device, int layout, int pipeline, int sequences, int draws);

    public synchronized int createRayPipeline(RayInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateRayPipeline(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateRayPipeline(long device, long info, int infoLength);

    public synchronized void destroyRayPipeline(int pipeline) {
        nDestroyRayPipeline(handle, pipeline);
    }

    private static native void nDestroyRayPipeline(long device, int pipeline);

    public synchronized byte[] rayHandles(int pipeline) {
        return nRayHandles(handle, pipeline);
    }

    private static native byte[] nRayHandles(long device, int pipeline);

    public synchronized int createSharedHeap(long size, int memory) {
        return nCreateSharedHeap(handle, size, memory);
    }

    private static native int nCreateSharedHeap(long device, long size, int memory);

    public synchronized int exportHeap(int heap) {
        return nExportHeap(handle, heap);
    }

    private static native int nExportHeap(long device, int heap);

    public synchronized int importHeap(int fd, long size, int memory) {
        return nImportHeap(handle, fd, size, memory);
    }

    private static native int nImportHeap(long device, int fd, long size, int memory);
}
