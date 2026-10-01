package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class VkDevice {

    public static Device adoptRaw(long proc, long instance, long physical, long device, int[] queues) {
        try (var stack = MemoryStack.stackPush()) {
            return new Device(nAdoptRaw(proc, instance, physical, device, memAddress(stack.ints(queues)), queues.length / 2));
        }
    }

    private static native long nAdoptRaw(long proc, long instance, long physical, long device, long queues, int queuesLength);

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
}
