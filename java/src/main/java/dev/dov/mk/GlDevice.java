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

    public Buffer importBuffer(int raw, long size) {
        lock.readLock().lock();
        try {
            return Buffer.of(nImportBuffer(handle(), raw, size));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportBuffer(long device, int raw, long size);

    public Texture importTexture(int raw, TextureInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Texture.of(nImportTexture(handle(), raw, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportTexture(long device, int raw, long info, int infoLength);

    public Texture importSharedTexture(long handle, TextureInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Texture.of(nImportSharedTexture(handle(), handle, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportSharedTexture(long device, long handle, long info, int infoLength);

    public Timeline importSharedTimeline(long handle) {
        lock.readLock().lock();
        try {
            return new Timeline(nImportSharedTimeline(handle(), handle));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportSharedTimeline(long device, long handle);

    public void waitShared(Timeline timeline, long value, Texture[] textures) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var texturesSize = 4;
                for (var x : textures) {
                    texturesSize += 8;
                }
                var texturesAddress = arena.alloc(texturesSize);
                var texturesAt = texturesAddress;
                memPutInt(texturesAt, textures.length);
                texturesAt += 4;
                for (var x : textures) {
                    memPutLong(texturesAt, Texture.raw(x));
                    texturesAt += 8;
                }
                nWaitShared(handle(), timeline.handle(arena), value, texturesAddress, texturesSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nWaitShared(long device, long timeline, long value, long textures, int texturesLength);

    public void signalShared(Timeline timeline, long value, Texture[] textures) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var texturesSize = 4;
                for (var x : textures) {
                    texturesSize += 8;
                }
                var texturesAddress = arena.alloc(texturesSize);
                var texturesAt = texturesAddress;
                memPutInt(texturesAt, textures.length);
                texturesAt += 4;
                for (var x : textures) {
                    memPutLong(texturesAt, Texture.raw(x));
                    texturesAt += 8;
                }
                nSignalShared(handle(), timeline.handle(arena), value, texturesAddress, texturesSize);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nSignalShared(long device, long timeline, long value, long textures, int texturesLength);

    public long deviceLuid() {
        lock.readLock().lock();
        try {
            return nDeviceLuid(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nDeviceLuid(long device);

    public RenderPass importRenderPass(CommandPool pool, ImportedPassInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return RenderPass.adopt(this, nImportRenderPass(handle(), CommandPool.raw(pool), infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportRenderPass(long device, long pool, long info, int infoLength);

    public int rawBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            return nRawBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native int nRawBuffer(long device, long buffer);

    public int rawTexture(Texture texture) {
        lock.readLock().lock();
        try {
            return nRawTexture(handle(), Texture.raw(texture));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native int nRawTexture(long device, long texture);
}
