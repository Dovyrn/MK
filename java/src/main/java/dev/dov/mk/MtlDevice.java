package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class MtlDevice extends Device {
    public MtlDevice(long handle) {
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

    public static MtlDevice importDeviceRaw(long device, long compiler, int queues) {
        return new MtlDevice(nImportDeviceRaw(device, compiler, queues));
    }

    private static native long nImportDeviceRaw(long device, long compiler, int queues);

    public Buffer importBuffer(long raw) {
        lock.readLock().lock();
        try {
            return Buffer.of(nImportBuffer(handle(), raw));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportBuffer(long device, long raw);

    public Texture importTexture(long raw, Format format) {
        lock.readLock().lock();
        try {
            return Texture.of(nImportTexture(handle(), raw, format.value));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportTexture(long device, long raw, int format);

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

    public ComputePass importComputePass(CommandPool pool, long raw) {
        lock.readLock().lock();
        try {
            return ComputePass.adopt(this, nImportComputePass(handle(), CommandPool.raw(pool), raw));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportComputePass(long device, long pool, long raw);

    public long rawResidencySet(ResidencySet set) {
        lock.readLock().lock();
        try {
            return nRawResidencySet(handle(), ResidencySet.raw(set));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawResidencySet(long device, long set);

    public long rawBuffer(Buffer buffer) {
        lock.readLock().lock();
        try {
            return nRawBuffer(handle(), Buffer.raw(buffer));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawBuffer(long device, long buffer);

    public long rawTexture(Texture texture) {
        lock.readLock().lock();
        try {
            return nRawTexture(handle(), Texture.raw(texture));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawTexture(long device, long texture);

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
