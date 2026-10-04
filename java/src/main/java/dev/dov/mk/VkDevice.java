package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class VkDevice extends Device {
    public VkDevice(long handle) {
        super(handle);
    }

    public synchronized int createHeap(long size, int memory) {
        return nCreateHeap(handle, size, memory);
    }

    private static native int nCreateHeap(long device, long size, int memory);

    public synchronized void destroyHeap(int heap) {
        nDestroyHeap(handle, heap);
    }

    private static native void nDestroyHeap(long device, int heap);

    public synchronized long[] bufferSize(long size, int memory) {
        try (var arena = new Arena()) {
            var out = arena.alloc(16);
            nBufferSize(handle, size, memory, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nBufferSize(long device, long size, int memory, long out);

    public synchronized long[] textureSize(TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            var out = arena.alloc(16);
            nTextureSize(handle, infoAddress, info.encodedSize(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nTextureSize(long device, long info, int infoLength, long out);

    public synchronized int createPlacedBuffer(int heap, long offset, long size) {
        return nCreatePlacedBuffer(handle, heap, offset, size);
    }

    private static native int nCreatePlacedBuffer(long device, int heap, long offset, long size);

    public synchronized int createPlacedTexture(int heap, long offset, TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreatePlacedTexture(handle, heap, offset, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreatePlacedTexture(long device, int heap, long offset, long info, int infoLength);

    public synchronized long textureHandle(int texture) {
        return nTextureHandle(handle, texture);
    }

    private static native long nTextureHandle(long device, int texture);

    public synchronized long samplerHandle(int sampler) {
        return nSamplerHandle(handle, sampler);
    }

    private static native long nSamplerHandle(long device, int sampler);

    public synchronized int createAccel(AccelInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateAccel(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateAccel(long device, long info, int infoLength);

    public synchronized void destroyAccel(int accel) {
        nDestroyAccel(handle, accel);
    }

    private static native void nDestroyAccel(long device, int accel);

    public synchronized long[] accelScratch(int accel) {
        try (var arena = new Arena()) {
            var out = arena.alloc(16);
            nAccelScratch(handle, accel, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nAccelScratch(long device, int accel, long out);

    public synchronized long accelHandle(int accel) {
        return nAccelHandle(handle, accel);
    }

    private static native long nAccelHandle(long device, int accel);

    public synchronized int createRayInstances(int count, int memory) {
        return nCreateRayInstances(handle, count, memory);
    }

    private static native int nCreateRayInstances(long device, int count, int memory);

    public synchronized void destroyRayInstances(int instances) {
        nDestroyRayInstances(handle, instances);
    }

    private static native void nDestroyRayInstances(long device, int instances);

    public synchronized void writeRayInstance(int instances, int index, int accel, float[] transform, int id, int mask, int flags) {
        try (var arena = new Arena()) {
            if (transform.length != 12) {
                throw new IllegalArgumentException("transform has 12 values");
            }
            nWriteRayInstance(handle, instances, index, accel, arena.floats(transform), id, mask, flags);
        }
    }

    private static native void nWriteRayInstance(long device, int instances, int index, int accel, long transform, int id, int mask, int flags);

    public synchronized long rayInstancesHandle(int instances) {
        return nRayInstancesHandle(handle, instances);
    }

    private static native long nRayInstancesHandle(long device, int instances);

    public static VkDevice adoptRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var arena = new Arena()) {
            if (queues.length % 2 != 0) {
                throw new IllegalArgumentException("queues holds pairs");
            }
            return new VkDevice(nAdoptRaw(proc, instance, physical, device, arena.ints(queues), queues.length / 2, features));
        }
    }

    private static native long nAdoptRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public synchronized int importBuffer(long raw, long size) {
        return nImportBuffer(handle, raw, size);
    }

    private static native int nImportBuffer(long device, long raw, long size);

    public synchronized int importTexture(long image, TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nImportTexture(handle, image, infoAddress, info.encodedSize());
        }
    }

    private static native int nImportTexture(long device, long image, long info, int infoLength);

    public synchronized Timeline importTimeline(long raw) {
        return new Timeline(nImportTimeline(handle, raw));
    }

    private static native long nImportTimeline(long device, long raw);

    public synchronized long rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native long nRawBuffer(long device, int buffer);

    public synchronized long[] rawTexture(int texture) {
        try (var arena = new Arena()) {
            var out = arena.alloc(16);
            nRawTexture(handle, texture, out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nRawTexture(long device, int texture, long out);

    public synchronized long rawQueue(int queue) {
        return nRawQueue(handle, queue);
    }

    private static native long nRawQueue(long device, int queue);

    public synchronized int createCompacted(int accel, long size) {
        return nCreateCompacted(handle, accel, size);
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public synchronized long sparsePage() {
        return nSparsePage(handle);
    }

    private static native long nSparsePage(long device);

    public synchronized int[] sparseTile(TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            var out = arena.alloc(12);
            nSparseTile(handle, infoAddress, info.encodedSize(), out);
            return new int[] {memGetInt(out + 0), memGetInt(out + 4), memGetInt(out + 8)};
        }
    }

    private static native void nSparseTile(long device, long info, int infoLength, long out);

    public synchronized int createSparseBuffer(long size) {
        return nCreateSparseBuffer(handle, size);
    }

    private static native int nCreateSparseBuffer(long device, long size);

    public synchronized int createSparseTexture(TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nCreateSparseTexture(handle, infoAddress, info.encodedSize());
        }
    }

    private static native int nCreateSparseTexture(long device, long info, int infoLength);

    public synchronized void mapSparseBuffer(int queue, int buffer, int heap, SparseRange[] ranges, Point[] waits, Point[] signals) {
        try (var arena = new Arena()) {
            var rangesSize = 4;
            for (var x : ranges) {
                rangesSize += x.encodedSize();
            }
            var rangesAddress = arena.alloc(rangesSize);
            var rangesAt = rangesAddress;
            memPutInt(rangesAt, ranges.length);
            rangesAt += 4;
            for (var x : ranges) {
                rangesAt = x.encode(rangesAt);
            }
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
            nMapSparseBuffer(handle, queue, buffer, heap, rangesAddress, rangesSize, waitsAddress, waits.length, signalsAddress, signals.length);
        }
    }

    private static native void nMapSparseBuffer(long device, int queue, int buffer, int heap, long ranges, int rangesLength, long waits, int waitsLength, long signals, int signalsLength);

    public synchronized void mapSparseTexture(int queue, int texture, int heap, SparseRegion[] regions, Point[] waits, Point[] signals) {
        try (var arena = new Arena()) {
            var regionsSize = 4;
            for (var x : regions) {
                regionsSize += x.encodedSize();
            }
            var regionsAddress = arena.alloc(regionsSize);
            var regionsAt = regionsAddress;
            memPutInt(regionsAt, regions.length);
            regionsAt += 4;
            for (var x : regions) {
                regionsAt = x.encode(regionsAt);
            }
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
            nMapSparseTexture(handle, queue, texture, heap, regionsAddress, regionsSize, waitsAddress, waits.length, signalsAddress, signals.length);
        }
    }

    private static native void nMapSparseTexture(long device, int queue, int texture, int heap, long regions, int regionsLength, long waits, int waitsLength, long signals, int signalsLength);
}
