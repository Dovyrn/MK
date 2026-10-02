package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.file.Files;
import java.nio.file.Path;

public class Smoke {
    public static void main(String[] args) throws Exception {
        System.load(Path.of(args[0]).toAbsolutePath().toString());
        Device device;
        if (args[0].contains("mk_gl")) {
            device = GlHost.device();
        } else if (args[0].contains("mk_vk")) {
            device = VulkanHost.device();
        } else {
            device = MetalHost.device();
        }
        var caps = device.caps();
        System.out.println("caps: mesh " + caps.mesh + ", push " + caps.push + ", ray " + caps.ray);
        if (!caps.compute) {
            record(Path.of(args[2]));
            device.close();
            System.out.println("java: no compute, stream recorded");
            return;
        }
        var n = 1024;
        var buffer = device.createBuffer(n * 4L, Memory.READBACK);
        var bytes = Files.readAllBytes(Path.of(args[1]));
        var code = memAlloc(bytes.length).put(bytes).flip();
        var info = new ComputeInfo();
        info.code = code;
        info.entry = "fill";
        info.group = new int[] {64, 1, 1};
        var pipeline = device.createPipeline(info);
        var event = device.createEvent();
        var push = memAlloc(8);
        push.putInt(0, n);
        push.putInt(4, 100);
        var c = new Commands();
        c.beginCompute();
        c.bindPipeline(pipeline);
        c.bindStorage(0, buffer, 0, n * 4L);
        c.pushConstants(0, push);
        c.dispatch(n / 64, 1, 1);
        c.barrier(Stage.COMPUTE, Access.SHADER_WRITE, Stage.HOST, Access.HOST_READ);
        c.endCompute();
        device.submit(0, c, new long[0], new long[] {event, 1});
        if (!device.wait(event, 1, 1000)) {
            throw new IllegalStateException("timeout");
        }
        var data = memIntBuffer(device.map(buffer), n);
        for (var i = 0; i < n; i++) {
            if (data.get(i) != 100 + i) {
                throw new IllegalStateException("value " + i + " is " + data.get(i));
            }
        }
        device.unmap(buffer);
        try {
            info.code = push;
            device.createPipeline(info);
            throw new IllegalStateException("bad code was accepted");
        } catch (RuntimeException e) {
            System.out.println("bad code rejected: " + e.getMessage().lines().findFirst().orElse(""));
        }
        record(Path.of(args[2]));
        device.close();
        System.out.println("java: 1024 values match");
    }

    static void record(Path out) throws Exception {
        var s = new Commands();
        s.beginCompute();
        s.bindPipeline(7);
        s.pushConstants(16, memAlloc(3).put((byte) 1).put((byte) 2).put((byte) 3).flip());
        s.dispatch(1, 2, 3);
        s.dispatchIndirect(4, 1L << 40);
        s.copyBuffer(1, 2, 3, 4, 5);
        s.fillBuffer(9, 8, 7, (byte) 0xab);
        s.barrier(1, 2, 3, 4);
        s.beginLabel("frame");
        s.endLabel();
        s.endCompute();
        var viewport = new Viewport();
        viewport.x = 0.5f;
        viewport.y = 1.5f;
        viewport.width = 64;
        viewport.height = 32;
        s.setViewport(viewport);
        s.drawIndexed(3, 1, 0, -5, 2);
        var barrier = new TextureBarrier();
        barrier.texture = 1;
        barrier.srcStage = 2;
        barrier.srcAccess = 3;
        barrier.dstStage = 4;
        barrier.dstAccess = 5;
        barrier.before = 6;
        barrier.after = 7;
        s.textureBarrier(barrier);
        var recorded = new byte[s.getSize()];
        memByteBuffer(s.getAddress(), s.getSize()).get(recorded);
        Files.write(out, recorded);
    }
}
