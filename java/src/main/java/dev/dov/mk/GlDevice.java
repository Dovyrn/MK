package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

import lombok.experimental.UtilityClass;

@UtilityClass
public class GlDevice {

    public static Device adoptRaw(long proc) {
        try (var stack = MemoryStack.stackPush()) {
            return new Device(nAdoptRaw(proc));
        }
    }

    private static native long nAdoptRaw(long proc);

    public static int importBuffer(Device device, int raw, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportBuffer(device.handle, raw, size);
        }
    }

    private static native int nImportBuffer(long device, int raw, long size);

    public static int importTexture(Device device, int raw, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nImportTexture(device.handle, raw, infoAddress, info.size());
        }
    }

    private static native int nImportTexture(long device, int raw, long info, int infoLength);

    public static int rawBuffer(Device device, int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawBuffer(device.handle, buffer);
        }
    }

    private static native int nRawBuffer(long device, int buffer);

    public static int rawTexture(Device device, int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawTexture(device.handle, texture);
        }
    }

    private static native int nRawTexture(long device, int texture);
}
