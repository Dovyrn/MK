package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class VkDevice extends Device {
    public VkDevice(long handle) {
        super(handle);
    }

    public long bindlessTexture(Texture texture) {
        lock.readLock().lock();
        try {
            return nBindlessTexture(handle(), Texture.raw(texture));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nBindlessTexture(long device, long texture);

    public long bindlessSampler(Sampler sampler) {
        lock.readLock().lock();
        try {
            return nBindlessSampler(handle(), Sampler.raw(sampler));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nBindlessSampler(long device, long sampler);

    public static DeviceInfo supportedFeatures() {
        try (var arena = new Arena()) {
            var out = arena.alloc(DeviceInfo.SIZE);
            nSupportedFeatures(out);
            return DeviceInfo.decode(out);
        }
    }

    private static native void nSupportedFeatures(long out);

    public static VkDevice createDevice(DeviceInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return new VkDevice(nCreateDevice(infoAddress, info.encodedSize()));
        }
    }

    private static native long nCreateDevice(long info, int infoLength);

    public static VkDevice importDeviceRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var arena = new Arena()) {
            if (queues.length % 2 != 0) {
                throw new IllegalArgumentException("queues holds pairs");
            }
            return new VkDevice(nImportDeviceRaw(proc, instance, physical, device, arena.ints(queues), queues.length / 2, features));
        }
    }

    private static native long nImportDeviceRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public Buffer importBuffer(long raw, long size, long data) {
        lock.readLock().lock();
        try {
            return Buffer.of(nImportBuffer(handle(), raw, size, data));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportBuffer(long device, long raw, long size, long data);

    public Texture importTexture(long image, TextureInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Texture.of(nImportTexture(handle(), image, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportTexture(long device, long image, long info, int infoLength);

    public Timeline importTimeline(long raw) {
        lock.readLock().lock();
        try {
            return new Timeline(nImportTimeline(handle(), raw));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportTimeline(long device, long raw);

    public CommandBuffer importCommands(CommandPool pool, long raw) {
        lock.readLock().lock();
        try {
            return CommandBuffer.adopt(this, nImportCommands(handle(), CommandPool.raw(pool), raw));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportCommands(long device, long pool, long raw);

    public RenderPass importRenderPass(CommandPool pool, long raw, ImportedPassInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return RenderPass.adopt(this, nImportRenderPass(handle(), CommandPool.raw(pool), raw, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportRenderPass(long device, long pool, long raw, long info, int infoLength);

    public long rawBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            return nRawBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawBuffer(long device, long buffer);

    public long[] rawTexture(Texture texture) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var out = arena.alloc(16);
                nRawTexture(handle(), Texture.raw(texture), out);
                return new long[] {memGetLong(out), memGetLong(out + 8)};
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nRawTexture(long device, long texture, long out);

    public long rawQueue(int queue) {
        lock.readLock().lock();
        try {
            return nRawQueue(handle(), queue);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawQueue(long device, int queue);
}
