package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class AccelInfo {
    public Triangles[] triangles = new Triangles[0];
    public Boxes[] boxes = new Boxes[0];
    public int instances = -1;
    public long instanceOffset = 0;
    public int instanceCount = 0;
    public int flags = 0;

    int encodedSize() {
        var trianglesSize = 4;
        for (var x : triangles) {
            trianglesSize += x.encodedSize();
        }
        var boxesSize = 4;
        for (var x : boxes) {
            boxesSize += x.encodedSize();
        }
        return 20 + trianglesSize + boxesSize;
    }

    long encode(long p) {
        memPutInt(p, triangles.length);
        p += 4;
        for (var x : triangles) {
            p = x.encode(p);
        }
        memPutInt(p, boxes.length);
        p += 4;
        for (var x : boxes) {
            p = x.encode(p);
        }
        memPutInt(p, instances);
        p += 4;
        memPutLong(p, instanceOffset);
        p += 8;
        memPutInt(p, instanceCount);
        p += 4;
        memPutInt(p, flags);
        p += 4;
        return p;
    }
}
