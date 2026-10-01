package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

public class RayInfo {
    public ByteBuffer code = null;
    public String raygen = "";
    public String[] misses = new String[0];
    public String[] closest = new String[0];
    public String[] any = new String[0];
    public String[] intersection = new String[0];
    public String[] callables = new String[0];
    public int depth = 1;

    public int size() {
        var missesSize = 0;
        for (var x : misses) {
            missesSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        var closestSize = 0;
        for (var x : closest) {
            closestSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        var anySize = 0;
        for (var x : any) {
            anySize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        var intersectionSize = 0;
        for (var x : intersection) {
            intersectionSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        var callablesSize = 0;
        for (var x : callables) {
            callablesSize += 4 + x.getBytes(StandardCharsets.UTF_8).length;
        }
        return 44 + raygen.getBytes(StandardCharsets.UTF_8).length + missesSize + closestSize + anySize + intersectionSize + callablesSize;
    }

    public long write(long p) {
        memPutLong(p, code == null ? 0 : memAddress(code));
        memPutLong(p + 8, code == null ? 0 : code.remaining());
        p += 16;
        var raygenBytes = raygen.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, raygenBytes.length);
        memByteBuffer(p + 4, raygenBytes.length).put(raygenBytes);
        p += 4 + raygenBytes.length;
        memPutInt(p, misses.length);
        p += 4;
        for (var x : misses) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, closest.length);
        p += 4;
        for (var x : closest) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, any.length);
        p += 4;
        for (var x : any) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, intersection.length);
        p += 4;
        for (var x : intersection) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, callables.length);
        p += 4;
        for (var x : callables) {
            var b = x.getBytes(StandardCharsets.UTF_8);
            memPutInt(p, b.length);
            memByteBuffer(p + 4, b.length).put(b);
            p += 4 + b.length;
        }
        memPutInt(p, depth);
        p += 4;
        return p;
    }
}
