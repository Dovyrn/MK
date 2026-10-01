package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

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
    public VertexBuffer[] buffers = new VertexBuffer[0];
    public VertexAttribute[] attributes = new VertexAttribute[0];

    public int size() {
        var colorsSize = 0;
        for (var x : colors) {
            colorsSize += x.size();
        }
        var buffersSize = 0;
        for (var x : buffers) {
            buffersSize += x.size();
        }
        var attributesSize = 0;
        for (var x : attributes) {
            attributesSize += x.size();
        }
        return 64 + vertex.getBytes(StandardCharsets.UTF_8).length + fragment.getBytes(StandardCharsets.UTF_8).length + colorsSize + buffersSize + attributesSize;
    }

    public long write(long p) {
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
            p = x.write(p);
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
        memPutInt(p, buffers.length);
        p += 4;
        for (var x : buffers) {
            p = x.write(p);
        }
        memPutInt(p, attributes.length);
        p += 4;
        for (var x : attributes) {
            p = x.write(p);
        }
        return p;
    }
}
