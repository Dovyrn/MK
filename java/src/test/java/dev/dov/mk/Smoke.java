package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.file.Files;
import java.nio.file.Path;

public class Smoke {
    public static void main(String[] args) throws Exception {
        System.load(Path.of(args[0]).toAbsolutePath().toString());
        try (var device = device(args[0])) {
            var caps = device.caps();
            System.out.println("caps: mesh " + caps.mesh + ", slots " + caps.slotBinding + ", tiles " + caps.tiles);
            clear(device);
            if (!caps.compute) {
                System.out.println("java: no compute");
                return;
            }
            var n = 1024;
            var buffer = device.createBuffer(n * 4L, Memory.READBACK);
            var bytes = Files.readAllBytes(Path.of(args[1]));
            var code = nmemAlloc(bytes.length);
            memByteBuffer(code, bytes.length).put(bytes);
            var info = new ComputePipelineInfo()
                    .code(code)
                    .codeSize(bytes.length)
                    .entry("fill");
            var pipeline = device.createComputePipeline(info);
            var timeline = device.createTimeline();
            var pool = device.createCommandPool(0);
            var constants = device.createBuffer(8, Memory.UPLOAD);
            var push = device.mapBuffer(constants);
            memPutInt(push, n);
            memPutInt(push + 4, 100);
            device.unmapBuffer(constants);
            var set = device.createResidencySet();
            device.addResidency(set, new Resource[] {buffer.resource(), constants.resource()});
            device.commitResidency(set);
            var commands = device.beginCommands(pool);
            commands.useResidencySet(set);
            try (var pass = commands.beginComputePass()) {
                pass.bindComputePipeline(pipeline);
                pass.bindStorageBuffer(0, buffer, 0, n * 4L);
                pass.bindUniformBuffer(0, constants, 0, 8);
                pass.dispatch(n / 64, 1, 1);
                pass.memoryBarrier(Stage.COMPUTE, Stage.HOST);
            }
            try {
                device.createBuffer(4, Memory.UPLOAD);
                throw new IllegalStateException("a device change was accepted while recording");
            } catch (IllegalStateException e) {
                System.out.println("change while recording rejected");
            }
            device.submit(new CommandBuffer[] {commands}, new TimelinePoint[0], new TimelinePoint[] {new TimelinePoint(timeline, 1)});
            if (!timeline.await(1, 1000)) {
                throw new IllegalStateException("timeout");
            }
            var data = memIntBuffer(device.mapBuffer(buffer), n);
            for (var i = 0; i < n; i++) {
                if (data.get(i) != 100 + i) {
                    throw new IllegalStateException("value " + i + " is " + data.get(i));
                }
            }
            device.unmapBuffer(buffer);
            try {
                device.createComputePipeline(info.code(code).codeSize(8));
                throw new IllegalStateException("bad code was accepted");
            } catch (RuntimeException e) {
                System.out.println("bad code rejected: " + e.getMessage().lines().findFirst().orElse(""));
            }
            device.destroyCommandPool(pool);
            device.destroyTimeline(timeline);
            nmemFree(code);
            System.out.println("java: 1024 values match");
        }
    }

    static void clear(Device device) {
        var size = 4;
        var texture = device.createTexture(new TextureInfo()
                .format(Format.RGBA8_UNORM)
                .width(size)
                .height(size)
                .usage(TextureUsage.COLOR | TextureUsage.COPY_SRC));
        var pixels = device.createBuffer(size * size * 4L, Memory.READBACK);
        var timeline = device.createTimeline();
        var pool = device.createCommandPool(0);
        var set = device.createResidencySet();
        device.addResidency(set, new Resource[] {texture.resource(), pixels.resource()});
        device.commitResidency(set);
        var commands = device.beginCommands(pool);
        commands.useResidencySet(set);
        commands.textureBarrier(new TextureBarrier()
                .texture(texture)
                .srcStage(Stage.COLOR)
                .dstStage(Stage.COLOR)
                .before(TextureLayout.UNDEFINED));
        var color = new ColorAttachment().texture(texture).r(1).g(0.5f).b(0).a(1);
        try (var pass = commands.beginRenderPass(new RenderPassInfo()
                .width(size)
                .height(size)
                .colors(new ColorAttachment[] {color}))) {
        }
        commands.memoryBarrier(Stage.COLOR, Stage.COPY);
        try (var pass = commands.beginComputePass()) {
            pass.copyTextureToBuffer(new BufferTextureCopy()
                    .texture(texture)
                    .width(size)
                    .height(size)
                    .buffer(pixels));
            pass.memoryBarrier(Stage.COPY, Stage.HOST);
        }
        device.submit(new CommandBuffer[] {commands}, new TimelinePoint[0], new TimelinePoint[] {new TimelinePoint(timeline, 1)});
        if (!timeline.await(1, 1000)) {
            throw new IllegalStateException("timeout");
        }
        var data = memByteBuffer(device.mapBuffer(pixels), size * size * 4);
        for (var i = 0; i < size * size; i++) {
            var r = data.get(i * 4) & 0xff;
            var g = data.get(i * 4 + 1) & 0xff;
            if (r != 255 || Math.abs(g - 128) > 1 || data.get(i * 4 + 3) != (byte) 255) {
                throw new IllegalStateException("pixel " + i + " is " + r + ", " + g);
            }
        }
        device.unmapBuffer(pixels);
        device.destroyCommandPool(pool);
        device.destroyTimeline(timeline);
        System.out.println("java: render pass cleared and read back");
    }

    static Device device(String library) {
        if (library.contains("mk_gl")) {
            return GlHost.device();
        }
        if (library.contains("mk_vk")) {
            return VulkanHost.device();
        }
        return MetalHost.device();
    }
}
