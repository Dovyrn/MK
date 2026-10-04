package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class GlDevice extends Device {
    public GlDevice(long handle) {
        super(handle);
    }

    public static GlDevice adoptRaw(long proc) {
        return new GlDevice(nAdoptRaw(proc));
    }

    private static native long nAdoptRaw(long proc);

    public synchronized int importBuffer(int raw, long size) {
        return nImportBuffer(handle, raw, size);
    }

    private static native int nImportBuffer(long device, int raw, long size);

    public synchronized int importTexture(int raw, TextureInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return nImportTexture(handle, raw, infoAddress, info.encodedSize());
        }
    }

    private static native int nImportTexture(long device, int raw, long info, int infoLength);

    public synchronized int rawBuffer(int buffer) {
        return nRawBuffer(handle, buffer);
    }

    private static native int nRawBuffer(long device, int buffer);

    public synchronized int rawTexture(int texture) {
        return nRawTexture(handle, texture);
    }

    private static native int nRawTexture(long device, int texture);
}
