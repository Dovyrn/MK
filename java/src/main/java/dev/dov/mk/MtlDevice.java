package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class MtlDevice {

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
