package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class MeshPipelineInfo {
    public long code = 0;
    public long codeSize = 0;
    public String task = "";
    public String mesh = "";
    public String fragment = "";
    public ColorTarget[] colors = new ColorTarget[0];
    public int depth = Format.NONE;
    public int stencil = Format.NONE;
    public int samples = 1;
    public int fill = FillMode.FILL;
    public boolean coverage = false;
    public boolean clamp = false;
    public boolean ordered = false;

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 56 + task.getBytes(StandardCharsets.UTF_8).length + mesh.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize;
    }

    long encode(long p) {
        memPutLong(p, code);
        memPutLong(p + 8, codeSize);
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
        memPutInt(p, clamp ? 1 : 0);
        p += 4;
        memPutInt(p, ordered ? 1 : 0);
        p += 4;
        return p;
    }
}
