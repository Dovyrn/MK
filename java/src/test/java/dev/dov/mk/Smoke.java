package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.file.Files;
import java.nio.file.Path;

public class Smoke {
    public static void main(String[] args) throws Exception {
        System.load(Path.of(args[0]).toAbsolutePath().toString());
        var device = Device.create(1);
        var caps = device.caps();
        System.out.println("caps: mesh " + caps.mesh + ", push " + caps.push + ", ray " + caps.ray);
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
        var push = memAlloc(16);
        push.putLong(0, device.address(buffer));
        push.putInt(8, n);
        push.putInt(12, 100);
        var c = new Commands();
        c.beginCompute();
        c.bindPipeline(pipeline);
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
        try {
            info.code = push;
            device.createPipeline(info);
            throw new IllegalStateException("bad code was accepted");
        } catch (RuntimeException e) {
            System.out.println("bad code rejected: " + e.getMessage().lines().findFirst().orElse(""));
        }
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
        s.setViewport(0.5f, 1.5f, 64, 32, 0, 1);
        s.drawIndexed(3, 1, 0, -5, 2);
        s.textureBarrier(1, 2, 3, 4, 5, 6, 7, 0, -1, 0, -1);
        var recorded = new byte[s.getSize()];
        memByteBuffer(s.getAddress(), s.getSize()).get(recorded);
        Files.write(Path.of(args[2]), recorded);
        device.free();
        System.out.println("java: 1024 values match");
    }
}
