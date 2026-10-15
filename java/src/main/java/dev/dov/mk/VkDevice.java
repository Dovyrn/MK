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

    public void waitIdle(int queue) {
        lock.readLock().lock();
        try {
            nWaitIdle(handle(), queue);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nWaitIdle(long device, int queue);

    public void destroySwapchain(Swapchain swapchain) {
        lock.readLock().lock();
        try {
            nDestroySwapchain(handle(), Swapchain.raw(swapchain));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nDestroySwapchain(long device, long swapchain);

    public Texture acquireTexture(Swapchain swapchain) {
        lock.readLock().lock();
        try {
            return Texture.of(nAcquireTexture(handle(), Swapchain.raw(swapchain)));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nAcquireTexture(long device, long swapchain);

    public boolean submitAndPresent(Swapchain swapchain, CommandBuffer[] commands, TimelinePoint[] waits, TimelinePoint[] signals) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                for (var c : commands) {
                    c.handle();
                }
                var commandsAddress = arena.alloc(commands.length * 8L);
                for (var i = 0; i < commands.length; i++) {
                    memPutLong(commandsAddress + i * 8, commands[i].take());
                }
                var waitsAddress = arena.alloc(waits.length * 16L);
                for (var i = 0; i < waits.length; i++) {
                    memPutLong(waitsAddress + i * 16, waits[i].timeline().handle(arena));
                    memPutLong(waitsAddress + i * 16 + 8, waits[i].value());
                }
                var signalsAddress = arena.alloc(signals.length * 16L);
                for (var i = 0; i < signals.length; i++) {
                    memPutLong(signalsAddress + i * 16, signals[i].timeline().handle(arena));
                    memPutLong(signalsAddress + i * 16 + 8, signals[i].value());
                }
                return nSubmitAndPresent(handle(), Swapchain.raw(swapchain), commandsAddress, commands.length, waitsAddress, waits.length, signalsAddress, signals.length);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native boolean nSubmitAndPresent(long device, long swapchain, long commands, int commandsLength, long waits, int waitsLength, long signals, int signalsLength);

    public void recreateSwapchain(Swapchain swapchain, SwapchainInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                nRecreateSwapchain(handle(), Swapchain.raw(swapchain), infoAddress, info.encodedSize());
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nRecreateSwapchain(long device, long swapchain, long info, int infoLength);

    public boolean waitPresented(Swapchain swapchain, long present, long ms) {
        lock.readLock().lock();
        try {
            return nWaitPresented(handle(), Swapchain.raw(swapchain), present, ms);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native boolean nWaitPresented(long device, long swapchain, long present, long ms);

    public Swapchain createSwapchain(int queue, long surface, SwapchainInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Swapchain.of(nCreateSwapchain(handle(), queue, surface, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateSwapchain(long device, int queue, long surface, long info, int infoLength);

    public SurfaceCaps surfaceCaps(long surface) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var out = arena.alloc(SurfaceCaps.SIZE);
                nSurfaceCaps(handle(), surface, out);
                return SurfaceCaps.decode(out);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nSurfaceCaps(long device, long surface, long out);

    public boolean surfaceFormat(long surface, Format format, ColorSpace space) {
        lock.readLock().lock();
        try {
            return nSurfaceFormat(handle(), surface, format.value, space.value);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native boolean nSurfaceFormat(long device, long surface, int format, int space);

    public long rawInstance() {
        lock.readLock().lock();
        try {
            return nRawInstance(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nRawInstance(long device);

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

    public long deviceLuid() {
        lock.readLock().lock();
        try {
            return nDeviceLuid(handle());
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nDeviceLuid(long device);

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
