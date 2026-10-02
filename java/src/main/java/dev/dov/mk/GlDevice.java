package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.lang.ref.Reference;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.system.MemoryStack;

public class GlDevice extends Device {
    public GlDevice(long handle) {
        super(handle);
    }

    public static GlDevice adoptRaw(long proc) {
        try (var stack = MemoryStack.stackPush()) {
            return new GlDevice(nAdoptRaw(proc));
        }
    }

    private static native long nAdoptRaw(long proc);

    public int importBuffer(int raw, long size) {
        try (var stack = MemoryStack.stackPush()) {
            return nImportBuffer(handle, raw, size);
        }
    }

    private static native int nImportBuffer(long device, int raw, long size);

    public int importTexture(int raw, TextureInfo info) {
        try (var stack = MemoryStack.stackPush()) {
            var infoAddress = stack.nmalloc(8, info.size());
            info.write(infoAddress);
            return nImportTexture(handle, raw, infoAddress, info.size());
        } finally {
            Reference.reachabilityFence(info);
        }
    }

    private static native int nImportTexture(long device, int raw, long info, int infoLength);

    public int rawBuffer(int buffer) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawBuffer(handle, buffer);
        }
    }

    private static native int nRawBuffer(long device, int buffer);

    public int rawTexture(int texture) {
        try (var stack = MemoryStack.stackPush()) {
            return nRawTexture(handle, texture);
        }
    }

    private static native int nRawTexture(long device, int texture);
}
