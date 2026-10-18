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

    public static DeviceInfo supportedFeatures() {
        try (var arena = new Arena()) {
            var out = arena.alloc(DeviceInfo.SIZE);
            nSupportedFeatures(out);
            return DeviceInfo.decode(out);
        }
    }

    private static native void nSupportedFeatures(long out);

    public static MtlDevice createDevice(DeviceInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return new MtlDevice(nCreateDevice(infoAddress, info.encodedSize()));
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

    public boolean submitAndPresent(Swapchain[] swapchains, CommandBuffer[] commands, TimelinePoint[] waits, TimelinePoint[] signals) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var swapchainsSize = 4;
                for (var x : swapchains) {
                    swapchainsSize += 8;
                }
                var swapchainsAddress = arena.alloc(swapchainsSize);
                var swapchainsAt = swapchainsAddress;
                memPutInt(swapchainsAt, swapchains.length);
                swapchainsAt += 4;
                for (var x : swapchains) {
                    memPutLong(swapchainsAt, Swapchain.raw(x));
                    swapchainsAt += 8;
                }
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
                return nSubmitAndPresent(handle(), swapchainsAddress, swapchainsSize, commandsAddress, commands.length, waitsAddress, waits.length, signalsAddress, signals.length);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native boolean nSubmitAndPresent(long device, long swapchains, int swapchainsLength, long commands, int commandsLength, long waits, int waitsLength, long signals, int signalsLength);

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

    public SurfaceCaps surfaceCaps(long layer) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var out = arena.alloc(SurfaceCaps.SIZE);
                nSurfaceCaps(handle(), layer, out);
                return SurfaceCaps.decode(out);
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native void nSurfaceCaps(long device, long layer, long out);

    public boolean surfaceFormat(long layer, Format format, ColorSpace space) {
        lock.readLock().lock();
        try {
            return nSurfaceFormat(handle(), layer, format.value, space.value);
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native boolean nSurfaceFormat(long device, long layer, int format, int space);

    public Swapchain createSwapchain(int queue, long layer, SwapchainInfo info) {
        lock.readLock().lock();
        try {
            try (var arena = new Arena()) {
                var infoAddress = arena.alloc(info.encodedSize());
                info.encode(infoAddress);
                return Swapchain.of(nCreateSwapchain(handle(), queue, layer, infoAddress, info.encodedSize()));
            }
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nCreateSwapchain(long device, int queue, long layer, long info, int infoLength);

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

    public Sampler importSampler(long raw) {
        lock.readLock().lock();
        try {
            return Sampler.of(nImportSampler(handle(), raw));
        } finally {
            lock.readLock().unlock();
        }
    }

    private static native long nImportSampler(long device, long raw);

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
