package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class RenderInfo {
    public ByteBuffer code = null;
    public String vertex = "";
    public String fragment = "";
    public ColorTarget[] colors = new ColorTarget[0];
    public int depth = Format.NONE;
    public int stencil = Format.NONE;
    public int samples = 1;
    public int topology = Topology.TRIANGLE;
    public int fill = Fill.FILL;
    public boolean coverage = false;
    public int views = 0;
    public boolean indirect = false;
    public boolean clamp = false;
    public VertexBuffer[] buffers = new VertexBuffer[0];
    public VertexAttribute[] attributes = new VertexAttribute[0];

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
        return 60 + vertex.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize + buffersSize + attributesSize;
    }

    long encode(long p) {
        if (code != null && !code.isDirect()) {
            throw new IllegalArgumentException("code must be a direct ByteBuffer");
        }
        memPutLong(p, code == null ? 0 : memAddress(code));
        memPutLong(p + 8, code == null ? 0 : code.remaining());
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
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, stencil);
        p += 4;
        memPutInt(p, samples);
        p += 4;
        memPutInt(p, topology);
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
        return p;
    }
}
