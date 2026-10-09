package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class GlDevice extends Device {
    public GlDevice(long handle) {
        super(handle);
    }

    public static GlDevice importDeviceRaw(long proc) {
        return new GlDevice(nImportDeviceRaw(proc));
    }

    private static native long nImportDeviceRaw(long proc);

    public synchronized Buffer importBuffer(int raw, long size) {
        idle("importBuffer");
        return Buffer.of(nImportBuffer(handle(), raw, size));
    }

    private static native long nImportBuffer(long device, int raw, long size);

    public synchronized Texture importTexture(int raw, TextureInfo info) {
        idle("importTexture");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return Texture.of(nImportTexture(handle(), raw, infoAddress, info.encodedSize()));
        }
    }

    private static native long nImportTexture(long device, int raw, long info, int infoLength);

    public synchronized RenderPass importRenderPass(CommandPool pool, ImportedPassInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return RenderPass.adopt(this, nImportRenderPass(handle(), CommandPool.raw(pool), infoAddress, info.encodedSize()));
        }
    }

    private static native long nImportRenderPass(long device, long pool, long info, int infoLength);

    public synchronized int rawBuffer(Buffer buffer) {
        return nRawBuffer(handle(), Buffer.raw(buffer));
    }

    private static native int nRawBuffer(long device, long buffer);

    public synchronized int rawTexture(Texture texture) {
        return nRawTexture(handle(), Texture.raw(texture));
    }

    private static native int nRawTexture(long device, long texture);
}
