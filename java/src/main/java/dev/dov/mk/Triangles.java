package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class Triangles {
    public int vertices = 0;
    public long vertexOffset = 0;
    public int format = Format.RGB32_FLOAT;
    public int stride = 12;
    public int vertexCount = 0;
    public int indices = -1;
    public long indexOffset = 0;
    public int indexKind = Index.U32;
    public int count = 0;
    public boolean opaque = true;

    public int size() {
        return 48;
    }

    public long write(long p) {
        memPutInt(p, vertices);
        p += 4;
        memPutLong(p, vertexOffset);
        p += 8;
        memPutInt(p, format);
        p += 4;
        memPutInt(p, stride);
        p += 4;
        memPutInt(p, vertexCount);
        p += 4;
        memPutInt(p, indices);
        p += 4;
        memPutLong(p, indexOffset);
        p += 8;
        memPutInt(p, indexKind);
        p += 4;
        memPutInt(p, count);
        p += 4;
        memPutInt(p, opaque ? 1 : 0);
        p += 4;
        return p;
    }

    public static Triangles read(long p) {
        var r = new Triangles();
        r.vertices = memGetInt(p);
        p += 4;
        r.vertexOffset = memGetLong(p);
        p += 8;
        r.format = memGetInt(p);
        p += 4;
        r.stride = memGetInt(p);
        p += 4;
        r.vertexCount = memGetInt(p);
        p += 4;
        r.indices = memGetInt(p);
        p += 4;
        r.indexOffset = memGetLong(p);
        p += 8;
        r.indexKind = memGetInt(p);
        p += 4;
        r.count = memGetInt(p);
        p += 4;
        r.opaque = memGetInt(p) != 0;
        p += 4;
        return r;
    }
}
