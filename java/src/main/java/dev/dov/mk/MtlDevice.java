package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;

public class MtlDevice extends Device {
    public MtlDevice(long handle) {
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

    public static MtlDevice adoptRaw(long device, int queues) {
        return new MtlDevice(nAdoptRaw(device, queues));
    }

    private static native long nAdoptRaw(long device, int queues);

    public synchronized Buffer importBuffer(long raw) {
        idle("importBuffer");
        return Buffer.of(nImportBuffer(handle(), raw));
    }

    private static native long nImportBuffer(long device, long raw);

    public synchronized Texture importTexture(long raw, int format) {
        idle("importTexture");
        return Texture.of(nImportTexture(handle(), raw, format));
    }

    private static native long nImportTexture(long device, long raw, int format);

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

    public synchronized ComputePass importComputePass(CommandPool pool, long raw) {
        return ComputePass.adopt(this, nImportComputePass(handle(), CommandPool.raw(pool), raw));
    }

    private static native long nImportComputePass(long device, long pool, long raw);

    public synchronized long rawBuffer(Buffer buffer) {
        return nRawBuffer(handle(), Buffer.raw(buffer));
    }

    private static native long nRawBuffer(long device, long buffer);

    public synchronized long rawTexture(Texture texture) {
        return nRawTexture(handle(), Texture.raw(texture));
    }

    private static native long nRawTexture(long device, long texture);

    public synchronized long rawQueue(int queue) {
        return nRawQueue(handle(), queue);
    }

    private static native long nRawQueue(long device, int queue);
}
