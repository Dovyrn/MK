package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class RenderPipelineInfo {
    public long code = 0;
    public long codeSize = 0;
    public String vertex = "";
    public String fragment = "";
    public ColorTarget[] colors = new ColorTarget[0];
    public Format depth = null;
    public Format stencil = null;
    public int samples = 1;
    public Topology topology = Topology.TRIANGLE;
    public FillMode fill = FillMode.FILL;
    public boolean coverage = false;
    public boolean clamp = false;
    public boolean ordered = false;
    public boolean indirect = false;
    public VertexBufferLayout[] buffers = new VertexBufferLayout[0];
    public VertexAttribute[] attributes = new VertexAttribute[0];
    public DescriptorLayout[] sets = new DescriptorLayout[0];
    public String label = "";

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        var buffersSize = 4;
        for (var x : buffers) {
            buffersSize += x.encodedSize();
        }
        var attributesSize = 4;
        for (var x : attributes) {
            attributesSize += x.encodedSize();
        }
        var setsSize = 4;
        for (var x : sets) {
            setsSize += 8;
        }
        return 64 + vertex.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize + (depth == null ? 0 : 4) + (stencil == null ? 0 : 4) + buffersSize + attributesSize + setsSize + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutLong(p, code);
        memPutLong(p + 8, codeSize);
        p += 16;
        var vertexBytes = vertex.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, vertexBytes.length);
        memByteBuffer(p + 4, vertexBytes.length).put(vertexBytes);
        p += 4 + vertexBytes.length;
        var fragmentBytes = fragment.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, fragmentBytes.length);
        memByteBuffer(p + 4, fragmentBytes.length).put(fragmentBytes);
        p += 4 + fragmentBytes.length;
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
        memPutInt(p, topology.value);
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
        memPutInt(p, buffers.length);
        p += 4;
        for (var x : buffers) {
            p = x.encode(p);
        }
        memPutInt(p, attributes.length);
        p += 4;
        for (var x : attributes) {
            p = x.encode(p);
        }
        memPutInt(p, sets.length);
        p += 4;
        for (var x : sets) {
            memPutLong(p, DescriptorLayout.raw(x));
            p += 8;
        }
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
