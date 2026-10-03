package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class MtlDevice extends Device {
    public MtlDevice(long handle) {
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

    public static MtlDevice adoptRaw(long device, int queues) {
        return new MtlDevice(nAdoptRaw(device, queues));
    }

    private static native long nAdoptRaw(long device, int queues);

    public synchronized int importBuffer(long raw) {
        return nImportBuffer(handle, raw);
    }

    private static native int nImportBuffer(long device, long raw);

    public synchronized int importTexture(long raw, int format) {
        return nImportTexture(handle, raw, format);
    }

    private static native int nImportTexture(long device, long raw, int format);

    public synchronized int importEvent(long raw) {
        return nImportEvent(handle, raw);
    }

    private static native int nImportEvent(long device, long raw);

    public synchronized long rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native long nRawBuffer(long device, int buffer);

    public synchronized long rawTexture(int texture) {
        return nRawTexture(handle, texture);
    }

    private static native long nRawTexture(long device, int texture);

    public synchronized long rawEvent(int event) {
        return nRawEvent(handle, event);
    }

    private static native long nRawEvent(long device, int event);

    public synchronized long rawQueue(int queue) {
        return nRawQueue(handle, queue);
    }

    private static native long nRawQueue(long device, int queue);

    public synchronized int createTilePipeline(long code, long codeSize, String entry, int[] colors, int samples) {
        try (var stack = MemoryStack.stackPush()) {
            var entryBytes = stack.UTF8(entry, false);
            return nCreateTilePipeline(handle, code, codeSize, memAddress(entryBytes), entryBytes.remaining(), memAddress(stack.ints(colors)), colors.length, samples);
        }
    }

    private static native int nCreateTilePipeline(long device, long code, long codeLength, long entry, int entryLength, long colors, int colorsLength, int samples);

    public synchronized int createCompacted(int accel, long size) {
        return nCreateCompacted(handle, accel, size);
    }

    private static native int nCreateCompacted(long device, int accel, long size);

    public synchronized int createFunctionTable(int pipeline, String[] functions, boolean intersection) {
        try (var stack = MemoryStack.stackPush()) {
            var functionsSize = 4;
            for (var x : functions) {
                functionsSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
            }
            var functionsAddress = nmemAlloc(functionsSize);
            var functionsAt = functionsAddress;
            memPutInt(functionsAt, functions.length);
            functionsAt += 4;
            for (var x : functions) {
                var b = x.getBytes(StandardCharsets.UTF_8);
                memPutInt(functionsAt, b.length);
                memByteBuffer(functionsAt + 4, b.length).put(b);
                functionsAt += 4 + b.length;
            }
            try {
                return nCreateFunctionTable(handle, pipeline, functionsAddress, functionsSize, intersection);
            } finally {
                nmemFree(functionsAddress);
            }
        }
    }

    private static native int nCreateFunctionTable(long device, int pipeline, long functions, int functionsLength, boolean intersection);

    public synchronized void destroyFunctionTable(int table) {
        nDestroyFunctionTable(handle, table);
    }

    private static native void nDestroyFunctionTable(long device, int table);

    public synchronized long functionTableHandle(int table) {
        return nFunctionTableHandle(handle, table);
    }

    private static native long nFunctionTableHandle(long device, int table);

    public synchronized int createIndirect(IndirectInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateIndirect(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateIndirect(long device, long info, int infoLength);

    public synchronized void destroyIndirect(int indirect) {
        nDestroyIndirect(handle, indirect);
    }

    private static native void nDestroyIndirect(long device, int indirect);

    public synchronized long indirectHandle(int indirect) {
        return nIndirectHandle(handle, indirect);
    }

    private static native long nIndirectHandle(long device, int indirect);

    public synchronized int createRateMap(int width, int height, float[] horizontal, float[] vertical) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateRateMap(handle, width, height, memAddress(stack.floats(horizontal)), horizontal.length, memAddress(stack.floats(vertical)), vertical.length);
        }
    }

    private static native int nCreateRateMap(long device, int width, int height, long horizontal, int horizontalLength, long vertical, int verticalLength);

    public synchronized int[] rateMapSize(int map) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 8);
            nRateMapSize(handle, map, out);
            return new int[] {memGetInt(out), memGetInt(out + 4)};
        }
    }

    private static native void nRateMapSize(long device, int map, long out);

    public synchronized void destroyRateMap(int map) {
        nDestroyRateMap(handle, map);
    }

    private static native void nDestroyRateMap(long device, int map);

    public synchronized long[] createSharedTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            var out = stack.nmalloc(8, 16);
            try {
                nCreateSharedTexture(handle, infoAddress, info.encodedSize(), out);
                return new long[] {memGetLong(out), memGetLong(out + 8)};
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native void nCreateSharedTexture(long device, long info, int infoLength, long out);

    public synchronized int importSharedTexture(int id, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nImportSharedTexture(handle, id, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nImportSharedTexture(long device, int id, long info, int infoLength);

    public synchronized int createTensor(TensorInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = nmemAlloc(info.encodedSize());
            info.encode(infoAddress);
            try {
                return nCreateTensor(handle, infoAddress, info.encodedSize());
            } finally {
                nmemFree(infoAddress);
            }
        }
    }

    private static native int nCreateTensor(long device, long info, int infoLength);

    public synchronized void destroyTensor(int tensor) {
        nDestroyTensor(handle, tensor);
    }

    private static native void nDestroyTensor(long device, int tensor);

    public synchronized long tensorHandle(int tensor) {
        return nTensorHandle(handle, tensor);
    }

    private static native long nTensorHandle(long device, int tensor);

    public synchronized long mapTensor(int tensor) {
        return nMapTensor(handle, tensor);
    }

    private static native long nMapTensor(long device, int tensor);

    public synchronized int createMlPipeline(String path, String entry) {
        try (var stack = MemoryStack.stackPush()) {
            var pathBytes = stack.UTF8(path, false);
            var entryBytes = stack.UTF8(entry, false);
            return nCreateMlPipeline(handle, memAddress(pathBytes), pathBytes.remaining(), memAddress(entryBytes), entryBytes.remaining());
        }
    }

    private static native int nCreateMlPipeline(long device, long path, int pathLength, long entry, int entryLength);

    public synchronized void destroyMlPipeline(int pipeline) {
        nDestroyMlPipeline(handle, pipeline);
    }

    private static native void nDestroyMlPipeline(long device, int pipeline);

    public synchronized long mlHeapSize(int pipeline) {
        return nMlHeapSize(handle, pipeline);
    }

    private static native long nMlHeapSize(long device, int pipeline);
}
