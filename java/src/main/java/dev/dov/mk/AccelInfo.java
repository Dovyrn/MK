package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class AccelInfo {
    public Triangles[] triangles = new Triangles[0];
    public Boxes[] boxes = new Boxes[0];
    public int rayInstances = -1;
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
        return 8 + trianglesSize + boxesSize;
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
        memPutInt(p, rayInstances);
        p += 4;
        memPutInt(p, flags);
        p += 4;
        return p;
    }
}
