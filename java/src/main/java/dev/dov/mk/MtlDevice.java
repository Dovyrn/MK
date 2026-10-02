package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class MtlDevice {

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
        }
    }

    private static native void nWriteInstance(long device, long out, long outLength, int accel, long transform, int id, int mask, int flags);

    public static Device adoptRaw(long device, int queues) {
        try (var stack = MemoryStack.stackPush()) {
            return new Device(nAdoptRaw(device, queues));
        }
    }

    private static native long nAdoptRaw(long device, int queues);

    public static int importBuffer(Device device, long raw) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportBuffer(device.handle, raw);
        }
    }

    private static native int nImportBuffer(long device, long raw);

    public static int importTexture(Device device, long raw, int format) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportTexture(device.handle, raw, format);
        }
    }

    private static native int nImportTexture(long device, long raw, int format);

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

    public static long rawTexture(Device device, int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawTexture(device.handle, texture);
        }
    }

    private static native long nRawTexture(long device, int texture);

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

    public static int createTilePipeline(Device device, ByteBuffer code, String entry, int[] colors, int samples) {
        try (var stack = MemoryStack.stackPush()) {
            var entryBytes = stack.UTF8(entry, false);
            return nCreateTilePipeline(device.handle, memAddress(code), code.remaining(), memAddress(entryBytes), entryBytes.remaining(), memAddress(stack.ints(colors)), colors.length, samples);
        }
    }

    private static native int nCreateTilePipeline(long device, long code, long codeLength, long entry, int entryLength, long colors, int colorsLength, int samples);

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

    public static int createFunctionTable(Device device, int pipeline, String[] functions, boolean intersection) {
        try (var stack = MemoryStack.stackPush()) {
            var functionsSize = 4;
            for (var x : functions) {
                functionsSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
            }
            var functionsAddress = stack.nmalloc(8, functionsSize);
            memPutInt(functionsAddress, functions.length);
            var functionsAt = functionsAddress + 4;
            for (var x : functions) {
                var b = x.getBytes(StandardCharsets.UTF_8);
                memPutInt(functionsAt, b.length);
                memByteBuffer(functionsAt + 4, b.length).put(b);
                functionsAt += 4 + b.length;
            }
            return nCreateFunctionTable(device.handle, pipeline, functionsAddress, functionsSize, intersection);
        }
    }

    private static native int nCreateFunctionTable(long device, int pipeline, long functions, int functionsLength, boolean intersection);

    public static void destroyFunctionTable(Device device, int table) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyFunctionTable(device.handle, table);
        }
    }

    private static native void nDestroyFunctionTable(long device, int table);

    public static long functionTableHandle(Device device, int table) {
        try (var stack = MemoryStack.stackPush()) {
            return nFunctionTableHandle(device.handle, table);
        }
    }

    private static native long nFunctionTableHandle(long device, int table);

    public static int createIndirect(Device device, IndirectInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateIndirect(device.handle, infoAddress, info.size());
        }
    }

    private static native int nCreateIndirect(long device, long info, int infoLength);

    public static void destroyIndirect(Device device, int indirect) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyIndirect(device.handle, indirect);
        }
    }

    private static native void nDestroyIndirect(long device, int indirect);

    public static long indirectHandle(Device device, int indirect) {
        try (var stack = MemoryStack.stackPush()) {
            return nIndirectHandle(device.handle, indirect);
        }
    }

    private static native long nIndirectHandle(long device, int indirect);

    public static int createRateMap(Device device, int width, int height, float[] horizontal, float[] vertical) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateRateMap(device.handle, width, height, memAddress(stack.floats(horizontal)), horizontal.length, memAddress(stack.floats(vertical)), vertical.length);
        }
    }

    private static native int nCreateRateMap(long device, int width, int height, long horizontal, int horizontalLength, long vertical, int verticalLength);

    public static int[] rateMapSize(Device device, int map) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 256);
            nRateMapSize(device.handle, map, out);
            return new int[] {memGetInt(out), memGetInt(out + 4)};
        }
    }

    private static native void nRateMapSize(long device, int map, long out);

    public static void destroyRateMap(Device device, int map) {
        try (var stack = MemoryStack.stackPush()) {
            nDestroyRateMap(device.handle, map);
        }
    }

    private static native void nDestroyRateMap(long device, int map);

    public static long[] createSharedTexture(Device device, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            var out = stack.nmalloc(8, 256);
            nCreateSharedTexture(device.handle, infoAddress, info.size(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nCreateSharedTexture(long device, long info, int infoLength, long out);

    public static int importSharedTexture(Device device, int id, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nImportSharedTexture(device.handle, id, infoAddress, info.size());
        }
    }

    private static native int nImportSharedTexture(long device, int id, long info, int infoLength);
}
