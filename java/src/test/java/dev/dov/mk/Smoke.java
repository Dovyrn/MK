package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.function.BiConsumer;
import java.util.function.Supplier;
import org.lwjgl.vulkan.KHRSurface;

public class Smoke {
    public static void main(String[] args) throws Exception {
        System.load(Path.of(args[0]).toAbsolutePath().toString());
        try (var device = device(args[0])) {
            var caps = device.capabilities();
            System.out.println("caps: mesh " + caps.meshShaders + ", storage sets " + caps.storageSets + ", tiles " + caps.tileShaders);
            clear(device);
            if (!caps.computeShaders) {
                System.out.println("java: no compute");
                return;
            }
            var n = 1024;
            var pool = device.createCommandPool(0);
            var commands = device.beginCommands(pool);
            var buffer = device.createBuffer(n * 4L, Memory.READBACK, BufferUsage.STORAGE, "smoke");
            var bytes = Files.readAllBytes(Path.of(args[1]));
            var code = nmemAlloc(bytes.length);
            memByteBuffer(code, bytes.length).put(bytes);
            var layout = device.createDescriptorLayout(new DescriptorBinding[] {
                new DescriptorBinding().binding(0).kind(DescriptorType.UNIFORM_BUFFER),
                new DescriptorBinding().binding(1).kind(DescriptorType.STORAGE_BUFFER)
            });
            var info = new ComputePipelineInfo()
                    .bytecode(code)
                    .bytecodeSize(bytes.length)
                    .entrypoint("fill")
                    .descriptorSetLayout(new DescriptorLayout[] {layout});
            var pipeline = device.createComputePipeline(info);
            var timeline = device.createTimeline();
            var constants = device.createBuffer(8, Memory.UPLOAD, BufferUsage.UNIFORM, "");
            var push = device.mapBuffer(constants);
            memPutInt(push, n);
            memPutInt(push + 4, 100);
            device.unmapBuffer(constants);
            var sets = device.createDescriptorPool(layout, 1);
            var bound = device.allocateDescriptorSet(sets);
            device.writeDescriptorSet(bound, new DescriptorWrite[] {
                new DescriptorWrite().binding(0).buffer(constants).bufferSize(8),
                new DescriptorWrite().binding(1).buffer(buffer).bufferSize(n * 4L)
            });
            var set = device.createResidencySet();
            device.addResidency(set, new ResidentResource[] {buffer.resource(), constants.resource()});
            device.commitResidency(set);
            commands.useResidencySet(set);
            var compute = commands.beginComputePass(0, 0);
            compute.bindComputePipeline(pipeline);
            compute.bindDescriptorSet(bound);
            compute.dispatch(n / 64, 1, 1);
            compute.memoryBarrier(PipelineStages.COMPUTE, PipelineStages.HOST);
            compute.end();
            var spare = device.createBuffer(4, Memory.UPLOAD, BufferUsage.COPY_DST, "");
            device.destroyBuffer(spare);
            System.out.println("created and destroyed while recording");
            commands.end();
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
                device.createComputePipeline(info.bytecode(code).bytecodeSize(8));
                throw new IllegalStateException("bad code was accepted");
            } catch (ShaderException e) {
                System.out.println("bad code rejected: " + e.getMessage().lines().findFirst().orElse(""));
            }
            threads(device, timeline);
            device.destroyCommandPool(pool);
            device.destroyTimeline(timeline);
            nmemFree(code);
            System.out.println("java: 1024 values match");
        }
        present(args[0]);
    }

    static SwapchainInfo swapchainInfo(int size) {
        return new SwapchainInfo()
                .width(size)
                .height(size)
                .usage(TextureUsage.COLOR_ATTACHMENT | TextureUsage.COPY_SRC);
    }

    static void present(String library) {
        var size = 16;
        if (library.contains("mk_vk") && !(VkDevice.supportedFeatures().swapchain && VulkanHost.headless())) {
            System.out.println("java: no headless surface");
        } else if (library.contains("mk_vk")) {
            try (var device = VkDevice.createDevice(VkDevice.supportedFeatures().swapchain(true))) {
                var instance = VulkanHost.wrap(device.rawInstance());
                var surface = VulkanHost.headless(instance);
                var swapchain = device.createSwapchain(0, surface, swapchainInfo(size));
                frames(device, size, () -> device.acquireTexture(swapchain), (c, signal) -> device.submitAndPresent(
                        new Swapchain[] {swapchain}, new CommandBuffer[] {c}, new TimelinePoint[0], new TimelinePoint[] {signal}));
                device.waitIdle(0);
                device.destroySwapchain(swapchain);
                KHRSurface.vkDestroySurfaceKHR(instance, surface, null);
            }
        } else if (!library.contains("mk_gl")) {
            try (var device = MtlDevice.createDevice(MtlDevice.supportedFeatures().swapchain(true))) {
                var layer = MetalHost.layer();
                var swapchain = device.createSwapchain(0, layer, swapchainInfo(size));
                frames(device, size, () -> device.acquireTexture(swapchain), (c, signal) -> device.submitAndPresent(
                        new Swapchain[] {swapchain}, new CommandBuffer[] {c}, new TimelinePoint[0], new TimelinePoint[] {signal}));
                device.waitIdle(0);
                device.destroySwapchain(swapchain);
                MetalHost.release(layer);
            }
        }
    }

    static void frames(Device device, int size, Supplier<Texture> acquire, BiConsumer<CommandBuffer, TimelinePoint> present) {
        var pixels = device.createBuffer(size * size * 4L, Memory.READBACK, BufferUsage.COPY_DST, "");
        var timeline = device.createTimeline();
        var pool = device.createCommandPool(0);
        var set = device.createResidencySet();
        device.addResidency(set, new ResidentResource[] {pixels.resource()});
        device.commitResidency(set);
        var colors = new float[][] {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}};
        for (var frame = 1; frame <= colors.length; frame++) {
            var rgb = colors[frame - 1];
            var texture = acquire.get();
            var commands = device.beginCommands(pool);
            commands.useResidencySet(set);
            commands.discardTextureContents(new TextureContentDiscard[] {new TextureContentDiscard()
                    .texture(texture)
                    .srcStage(PipelineStages.COLOR_ATTACHMENT)
                    .dstStage(PipelineStages.COLOR_ATTACHMENT)});
            var color = new ColorAttachment().texture(texture).clear(new float[] {rgb[0], rgb[1], rgb[2], 1});
            commands.beginRenderPass(new RenderPassInfo()
                    .width(size)
                    .height(size)
                    .colors(new ColorAttachment[] {color}))
                    .end();
            var copy = commands.beginComputePass(PipelineStages.COLOR_ATTACHMENT, PipelineStages.COPY);
            copy.copyTextureToBuffer(new BufferTextureCopy()
                    .texture(texture)
                    .width(size)
                    .height(size)
                    .buffer(pixels));
            copy.memoryBarrier(PipelineStages.COPY, PipelineStages.HOST);
            copy.end();
            commands.presentBarrier(texture, PipelineStages.COPY);
            commands.end();
            present.accept(commands, new TimelinePoint(timeline, frame));
            if (!timeline.await(frame, 1000)) {
                throw new IllegalStateException("timeout");
            }
            var data = memByteBuffer(device.mapBuffer(pixels), 4);
            var bgra = new int[] {(int) (rgb[2] * 255), (int) (rgb[1] * 255), (int) (rgb[0] * 255)};
            for (var i = 0; i < 3; i++) {
                if ((data.get(i) & 0xff) != bgra[i]) {
                    throw new IllegalStateException("frame " + frame + " channel " + i + " is " + (data.get(i) & 0xff));
                }
            }
            device.unmapBuffer(pixels);
        }
        device.destroyCommandPool(pool);
        device.destroyTimeline(timeline);
        System.out.println("java: 3 frames presented and read back");
    }

    static void threads(Device device, Timeline timeline) throws Exception {
        var size = 256;
        var buffers = new Buffer[2];
        var commands = new CommandBuffer[2];
        var workers = new Thread[2];
        for (var i = 0; i < 2; i++) {
            var at = i;
            workers[i] = new Thread(() -> {
                var pool = device.createCommandPool(0);
                var buffer = device.createBuffer(size, Memory.READBACK, BufferUsage.COPY_DST, "");
                var set = device.createResidencySet();
                device.addResidency(set, new ResidentResource[] {buffer.resource()});
                device.commitResidency(set);
                var c = device.beginCommands(pool);
                c.useResidencySet(set);
                var pass = c.beginComputePass(0, 0);
                pass.fillBuffer(buffer, 0, size, at + 1);
                pass.memoryBarrier(PipelineStages.COPY, PipelineStages.HOST);
                pass.end();
                c.end();
                buffers[at] = buffer;
                commands[at] = c;
            });
            workers[i].start();
        }
        for (var w : workers) {
            w.join();
        }
        device.submit(commands, new TimelinePoint[0], new TimelinePoint[] {new TimelinePoint(timeline, 2)});
        if (!timeline.await(2, 1000)) {
            throw new IllegalStateException("timeout");
        }
        for (var i = 0; i < 2; i++) {
            var data = memByteBuffer(device.mapBuffer(buffers[i]), size);
            for (var j = 0; j < size; j++) {
                if (data.get(j) != i + 1) {
                    throw new IllegalStateException("thread " + i + " byte " + j + " is " + data.get(j));
                }
            }
            device.unmapBuffer(buffers[i]);
        }
        System.out.println("java: 2 threads recorded and submitted together");
    }

    static void clear(Device device) {
        var size = 4;
        var texture = device.createTexture(new TextureInfo()
                .format(Format.RGBA8_UNORM)
                .width(size)
                .height(size)
                .usage(TextureUsage.COLOR_ATTACHMENT | TextureUsage.COPY_SRC));
        var pixels = device.createBuffer(size * size * 4L, Memory.READBACK, BufferUsage.COPY_DST, "");
        var timeline = device.createTimeline();
        var pool = device.createCommandPool(0);
        var set = device.createResidencySet();
        device.addResidency(set, new ResidentResource[] {texture.resource(), pixels.resource()});
        device.commitResidency(set);
        var commands = device.beginCommands(pool);
        commands.useResidencySet(set);
        commands.discardTextureContents(new TextureContentDiscard[] {new TextureContentDiscard()
                .texture(texture)
                .dstStage(PipelineStages.COLOR_ATTACHMENT)});
        var color = new ColorAttachment().texture(texture).clear(new float[] {1, 0.5f, 0, 1});
        commands.beginRenderPass(new RenderPassInfo()
                .width(size)
                .height(size)
                .colors(new ColorAttachment[] {color}))
                .end();
        var copy = commands.beginComputePass(PipelineStages.COLOR_ATTACHMENT, PipelineStages.COPY);
        copy.copyTextureToBuffer(new BufferTextureCopy()
                .texture(texture)
                .width(size)
                .height(size)
                .buffer(pixels));
        copy.memoryBarrier(PipelineStages.COPY, PipelineStages.HOST);
        copy.end();
        commands.end();
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
