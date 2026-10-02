package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class MeshInfo {
    public ByteBuffer code = null;
    public String task = "";
    public String mesh = "";
    public String fragment = "";
    public ColorTarget[] colors = new ColorTarget[0];
    public int depth = Format.NONE;
    public int stencil = Format.NONE;
    public int samples = 1;
    public int fill = Fill.FILL;
    public boolean coverage = false;
    public int views = 0;
    public boolean indirect = false;
    public boolean clamp = false;
    public int[] taskGroup = new int[] {1, 1, 1};
    public int[] meshGroup = new int[] {1, 1, 1};

    int encodedSize() {
        var colorsSize = 0;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 88 + task.getBytes(StandardCharsets.UTF_8).length + mesh.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize;
    }

    long encode(long p) {
        if (code != null && !code.isDirect()) {
            throw new IllegalArgumentException("code must be a direct ByteBuffer");
        }
        memPutLong(p, code == null ? 0 : memAddress(code));
        memPutLong(p + 8, code == null ? 0 : code.remaining());
        p += 16;
        var taskBytes = task.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, taskBytes.length);
        memByteBuffer(p + 4, taskBytes.length).put(taskBytes);
        p += 4 + taskBytes.length;
        var meshBytes = mesh.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, meshBytes.length);
        memByteBuffer(p + 4, meshBytes.length).put(meshBytes);
        p += 4 + meshBytes.length;
        var fragmentBytes = fragment.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, fragmentBytes.length);
        memByteBuffer(p + 4, fragmentBytes.length).put(fragmentBytes);
        p += 4 + fragmentBytes.length;
        memPutInt(p, colors.length);
        p += 4;
        for (var x : colors) {
            p = x.encode(p);
        }
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, stencil);
        p += 4;
        memPutInt(p, samples);
        p += 4;
        memPutInt(p, fill);
        p += 4;
        memPutInt(p, coverage ? 1 : 0);
        p += 4;
        memPutInt(p, views);
        p += 4;
        memPutInt(p, indirect ? 1 : 0);
        p += 4;
        memPutInt(p, clamp ? 1 : 0);
        p += 4;
        for (var x : taskGroup) {
            memPutInt(p, x);
            p += 4;
        }
        for (var x : meshGroup) {
            memPutInt(p, x);
            p += 4;
        }
        return p;
    }
}
