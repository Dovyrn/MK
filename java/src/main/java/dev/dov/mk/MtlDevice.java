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
}
