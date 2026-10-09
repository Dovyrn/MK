package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class VkDevice extends Device {
    public VkDevice(long handle) {
        super(handle);
    }

    public synchronized long bindlessTexture(Texture texture) {
        return nBindlessTexture(handle(), Texture.raw(texture));
    }

    private static native long nBindlessTexture(long device, long texture);

    public synchronized long bindlessSampler(Sampler sampler) {
        return nBindlessSampler(handle(), Sampler.raw(sampler));
    }

    private static native long nBindlessSampler(long device, long sampler);

    public static VkDevice importDeviceRaw(long proc, long instance, long physical, long device, int[] queues, long features) {
        try (var arena = new Arena()) {
            if (queues.length % 2 != 0) {
                throw new IllegalArgumentException("queues holds pairs");
            }
            return new VkDevice(nImportDeviceRaw(proc, instance, physical, device, arena.ints(queues), queues.length / 2, features));
        }
    }

    private static native long nImportDeviceRaw(long proc, long instance, long physical, long device, long queues, int queuesLength, long features);

    public synchronized Buffer importBuffer(long raw, long size, long data) {
        idle("importBuffer");
        return Buffer.of(nImportBuffer(handle(), raw, size, data));
    }

    private static native long nImportBuffer(long device, long raw, long size, long data);

    public synchronized Texture importTexture(long image, TextureInfo info) {
        idle("importTexture");
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return Texture.of(nImportTexture(handle(), image, infoAddress, info.encodedSize()));
        }
    }

    private static native long nImportTexture(long device, long image, long info, int infoLength);

    public synchronized Timeline importTimeline(long raw) {
        idle("importTimeline");
        return new Timeline(nImportTimeline(handle(), raw));
    }

    private static native long nImportTimeline(long device, long raw);

    public synchronized CommandBuffer importCommands(CommandPool pool, long raw) {
        return CommandBuffer.adopt(this, nImportCommands(handle(), CommandPool.raw(pool), raw));
    }

    private static native long nImportCommands(long device, long pool, long raw);

    public synchronized RenderPass importRenderPass(CommandPool pool, long raw, ImportedPassInfo info) {
        try (var arena = new Arena()) {
            var infoAddress = arena.alloc(info.encodedSize());
            info.encode(infoAddress);
            return RenderPass.adopt(this, nImportRenderPass(handle(), CommandPool.raw(pool), raw, infoAddress, info.encodedSize()));
        }
    }

    private static native long nImportRenderPass(long device, long pool, long raw, long info, int infoLength);

    public synchronized long rawBuffer(Buffer buffer) {
        return nRawBuffer(handle(), Buffer.raw(buffer));
    }

    private static native long nRawBuffer(long device, long buffer);

    public synchronized long[] rawTexture(Texture texture) {
        try (var arena = new Arena()) {
            var out = arena.alloc(16);
            nRawTexture(handle(), Texture.raw(texture), out);
            return new long[] {memGetLong(out), memGetLong(out + 8)};
        }
    }

    private static native void nRawTexture(long device, long texture, long out);

    public synchronized long rawQueue(int queue) {
        return nRawQueue(handle(), queue);
    }

    private static native long nRawQueue(long device, int queue);
}
