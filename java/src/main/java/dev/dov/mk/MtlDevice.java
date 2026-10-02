package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.lang.ref.Reference;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class MtlDevice extends Device {
    public MtlDevice(long handle) {
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
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            var out = stack.nmalloc(8, 16);
            nTextureSize(handle, infoAddress, info.encodedSize(), out);
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
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            return nCreatePlacedTexture(handle, heap, offset, infoAddress, info.encodedSize());
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
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            return nCreateAccel(handle, infoAddress, info.encodedSize());
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

    public static MtlDevice adoptRaw(long device, int queues) {
        return new MtlDevice(nAdoptRaw(device, queues));
    }

    private static native long nAdoptRaw(long device, int queues);

    public int importBuffer(long raw) {
        return nImportBuffer(handle, raw);
    }

    private static native int nImportBuffer(long device, long raw);

    public int importTexture(long raw, int format) {
        return nImportTexture(handle, raw, format);
    }

    private static native int nImportTexture(long device, long raw, int format);

    public int importEvent(long raw) {
        return nImportEvent(handle, raw);
    }

    private static native int nImportEvent(long device, long raw);

    public long rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native long nRawBuffer(long device, int buffer);

    public long rawTexture(int texture) {
        return nRawTexture(handle, texture);
    }

    private static native long nRawTexture(long device, int texture);

    public long rawEvent(int event) {
        return nRawEvent(handle, event);
    }

    private static native long nRawEvent(long device, int event);

    public long rawQueue(int queue) {
        return nRawQueue(handle, queue);
    }

    private static native long nRawQueue(long device, int queue);

    public int createTilePipeline(ByteBuffer code, String entry, int[] colors, int samples) {
        try (var stack = MemoryStack.stackPush()) {
            if (!code.isDirect()) {
                throw new IllegalArgumentException("code must be a direct ByteBuffer");
            }
            var entryBytes = stack.UTF8(entry, false);
            return nCreateTilePipeline(handle, memAddress(code), code.remaining(), memAddress(entryBytes), entryBytes.remaining(), memAddress(stack.ints(colors)), colors.length, samples);
        } finally {
            Reference.reachabilityFence(code);
        }
    }

    private static native int nCreateTilePipeline(long device, long code, long codeLength, long entry, int entryLength, long colors, int colorsLength, int samples);

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
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            var out = stack.nmalloc(8, 12);
            nSparseTile(handle, infoAddress, info.encodedSize(), out);
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
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            return nCreateSparseTexture(handle, infoAddress, info.encodedSize());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateSparseTexture(long device, long info, int infoLength);

    public void mapSparseBuffer(int queue, int buffer, int heap, SparseRange[] ranges, long[] waits, long[] signals) {
        try (var stack = MemoryStack.stackPush()) {
            var rangesSize = 4;
            for (var x : ranges) {
                rangesSize += x.encodedSize();
            }
            var rangesAddress = stack.nmalloc(8, rangesSize);
            memPutInt(rangesAddress, ranges.length);
            var rangesAt = rangesAddress + 4;
            for (var x : ranges) {
                rangesAt = x.encode(rangesAt);
            }
            if (waits.length % 2 != 0) {
                throw new IllegalArgumentException("waits holds event and value pairs");
            }
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            if (signals.length % 2 != 0) {
                throw new IllegalArgumentException("signals holds event and value pairs");
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
                regionsSize += x.encodedSize();
            }
            var regionsAddress = stack.nmalloc(8, regionsSize);
            memPutInt(regionsAddress, regions.length);
            var regionsAt = regionsAddress + 4;
            for (var x : regions) {
                regionsAt = x.encode(regionsAt);
            }
            if (waits.length % 2 != 0) {
                throw new IllegalArgumentException("waits holds event and value pairs");
            }
            var waitsAddress = stack.nmalloc(8, waits.length * 8);
            for (var i = 0; i < waits.length / 2; i++) {
                memPutInt(waitsAddress + i * 16, (int) waits[i * 2]);
                memPutLong(waitsAddress + i * 16 + 8, waits[i * 2 + 1]);
            }
            if (signals.length % 2 != 0) {
                throw new IllegalArgumentException("signals holds event and value pairs");
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

    public int createFunctionTable(int pipeline, String[] functions, boolean intersection) {
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
            return nCreateFunctionTable(handle, pipeline, functionsAddress, functionsSize, intersection);
        }
    }

    private static native int nCreateFunctionTable(long device, int pipeline, long functions, int functionsLength, boolean intersection);

    public void destroyFunctionTable(int table) {
        nDestroyFunctionTable(handle, table);
    }

    private static native void nDestroyFunctionTable(long device, int table);

    public long functionTableHandle(int table) {
        return nFunctionTableHandle(handle, table);
    }

    private static native long nFunctionTableHandle(long device, int table);

    public int createIndirect(IndirectInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            return nCreateIndirect(handle, infoAddress, info.encodedSize());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nCreateIndirect(long device, long info, int infoLength);

    public void destroyIndirect(int indirect) {
        nDestroyIndirect(handle, indirect);
    }

    private static native void nDestroyIndirect(long device, int indirect);

    public long indirectHandle(int indirect) {
        return nIndirectHandle(handle, indirect);
    }

    private static native long nIndirectHandle(long device, int indirect);

    public int createRateMap(int width, int height, float[] horizontal, float[] vertical) {
        try (var stack = MemoryStack.stackPush()) {
            return nCreateRateMap(handle, width, height, memAddress(stack.floats(horizontal)), horizontal.length, memAddress(stack.floats(vertical)), vertical.length);
        }
    }

    private static native int nCreateRateMap(long device, int width, int height, long horizontal, int horizontalLength, long vertical, int verticalLength);

    public int[] rateMapSize(int map) {
        try (var stack = MemoryStack.stackPush()) {
            var out = stack.nmalloc(8, 8);
            nRateMapSize(handle, map, out);
            return new int[] {memGetInt(out), memGetInt(out + 4)};
        }
    }

    private static native void nRateMapSize(long device, int map, long out);

    public void destroyRateMap(int map) {
        nDestroyRateMap(handle, map);
    }

    private static native void nDestroyRateMap(long device, int map);

    public long[] createSharedTexture(TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            var out = stack.nmalloc(8, 16);
            nCreateSharedTexture(handle, infoAddress, info.encodedSize(), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native void nCreateSharedTexture(long device, long info, int infoLength, long out);

    public int importSharedTexture(int id, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.encodedSize());
            info.encode(infoAddress);
            return nImportSharedTexture(handle, id, infoAddress, info.encodedSize());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nImportSharedTexture(long device, int id, long info, int infoLength);
}
