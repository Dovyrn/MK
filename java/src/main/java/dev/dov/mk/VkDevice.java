package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class VkDevice {

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

    public static int createCommandsLayout(Device device, LayoutInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nCreateCommandsLayout(device.handle, infoAddress, info.size());
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
