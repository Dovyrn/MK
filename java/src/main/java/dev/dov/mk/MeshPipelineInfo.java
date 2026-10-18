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
    public int[] taskGroup = new int[] {0, 0, 0};
    public int[] meshGroup = new int[] {0, 0, 0};
    public ColorTarget[] colors = new ColorTarget[0];
    public Format depth = null;
    public Format stencil = null;
    public int samples = 1;
    public FillMode fill = FillMode.FILL;
    public boolean coverage = false;
    public boolean clamp = false;
    public boolean ordered = false;
    public boolean indirect = false;
    public String label = "";

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 88 + task.getBytes(StandardCharsets.UTF_8).length + mesh.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize + (depth == null ? 0 : 4) + (stencil == null ? 0 : 4) + label.getBytes(StandardCharsets.UTF_8).length;
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
        if (taskGroup.length != 3) {
            throw new IllegalArgumentException("taskGroup has 3 values");
        }
        for (var x : taskGroup) {
            memPutInt(p, x);
            p += 4;
        }
        if (meshGroup.length != 3) {
            throw new IllegalArgumentException("meshGroup has 3 values");
        }
        for (var x : meshGroup) {
            memPutInt(p, x);
            p += 4;
        }
        memPutInt(p, colors.length);
        p += 4;
        for (var x : colors) {
            p = x.encode(p);
        }
        memPutInt(p, depth == null ? 0 : 1);
        p += 4;
        if (depth != null) {
            memPutInt(p, depth.value);
            p += 4;
        }
        memPutInt(p, stencil == null ? 0 : 1);
        p += 4;
        if (stencil != null) {
            memPutInt(p, stencil.value);
            p += 4;
        }
        memPutInt(p, samples);
        p += 4;
        memPutInt(p, fill.value);
        p += 4;
        memPutInt(p, coverage ? 1 : 0);
        p += 4;
        memPutInt(p, clamp ? 1 : 0);
        p += 4;
        memPutInt(p, ordered ? 1 : 0);
        p += 4;
        memPutInt(p, indirect ? 1 : 0);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
