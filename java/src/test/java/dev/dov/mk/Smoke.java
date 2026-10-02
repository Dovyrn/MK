package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.file.Files;
import java.nio.file.Path;

public class Smoke {
    public static void main(String[] args) throws Exception {
        System.load(Path.of(args[0]).toAbsolutePath().toString());
        try (var device = device(args[0])) {
            var caps = device.caps();
            System.out.println("caps: mesh " + caps.mesh + ", push " + caps.push + ", ray " + caps.ray);
            if (!caps.compute) {
                record(Path.of(args[2]));
                System.out.println("java: no compute, stream recorded");
                return;
            }
            var n = 1024;
            var buffer = device.createBuffer(n * 4L, Memory.READBACK);
            var bytes = Files.readAllBytes(Path.of(args[1]));
            var code = nmemAlloc(bytes.length);
            memByteBuffer(code, bytes.length).put(bytes);
            var info = new ComputeInfo()
                    .code(code)
                    .codeSize(bytes.length)
                    .entry("fill")
                    .group(new int[] {64, 1, 1});
            var pipeline = device.createPipeline(info);
            var event = device.createEvent();
            var push = nmemAlloc(8);
            memPutInt(push, n);
            memPutInt(push + 4, 100);
            try (var c = new Commands()) {
                c.beginCompute();
                c.bindPipeline(pipeline);
                c.bindStorage(0, buffer, 0, n * 4L);
                c.pushConstants(0, push, 8);
                c.dispatch(n / 64, 1, 1);
                c.barrier(Stage.COMPUTE, Access.SHADER_WRITE, Stage.HOST, Access.HOST_READ);
                c.endCompute();
                device.submit(0, c, new long[0], new long[] {event, 1});
                if (!device.await(event, 1, 1000)) {
                    throw new IllegalStateException("timeout");
                }
            }
            var data = memIntBuffer(device.map(buffer), n);
            for (var i = 0; i < n; i++) {
                if (data.get(i) != 100 + i) {
                    throw new IllegalStateException("value " + i + " is " + data.get(i));
                }
            }
            device.unmap(buffer);
            try {
                device.createPipeline(info.code(push).codeSize(8));
                throw new IllegalStateException("bad code was accepted");
            } catch (RuntimeException e) {
                System.out.println("bad code rejected: " + e.getMessage().lines().findFirst().orElse(""));
            }
            nmemFree(code);
            nmemFree(push);
            record(Path.of(args[2]));
            System.out.println("java: 1024 values match");
        }
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

    static void record(Path out) throws Exception {
        try (var s = new Commands()) {
            s.beginCompute();
            s.bindPipeline(7);
            var push = nmemAlloc(3);
            memPutByte(push, (byte) 1);
            memPutByte(push + 1, (byte) 2);
            memPutByte(push + 2, (byte) 3);
            s.pushConstants(16, push, 3);
            nmemFree(push);
            s.dispatch(1, 2, 3);
            s.dispatchIndirect(4, 1L << 40);
            s.copyBuffer(1, 2, 3, 4, 5);
            s.fillBuffer(9, 8, 7, (byte) 0xab);
            s.barrier(1, 2, 3, 4);
            s.beginLabel("frame");
            s.endLabel();
            s.endCompute();
            s.setViewport(new Viewport().x(0.5f).y(1.5f).width(64).height(32));
            s.drawIndexed(3, 1, 0, -5, 2);
            s.textureBarrier(new TextureBarrier()
                    .texture(1)
                    .srcStage(2)
                    .srcAccess(3)
                    .dstStage(4)
                    .dstAccess(5)
                    .before(6)
                    .after(7));
            var recorded = new byte[s.getSize()];
            memByteBuffer(s.getAddress(), s.getSize()).get(recorded);
            Files.write(out, recorded);
        }
    }
}
