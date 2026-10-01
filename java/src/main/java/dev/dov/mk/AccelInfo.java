package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class AccelInfo {
    public Triangles[] triangles = new Triangles[0];
    public Boxes[] boxes = new Boxes[0];
    public int instances = -1;
    public long instanceOffset = 0;
    public int instanceCount = 0;
    public int flags = 0;

    public int size() {
        var trianglesSize = 0;
        for (var x : triangles) {
            trianglesSize += x.size();
        }
        var boxesSize = 0;
        for (var x : boxes) {
            boxesSize += x.size();
        }
        return 28 + trianglesSize + boxesSize;
    }

    public long write(long p) {
        memPutInt(p, triangles.length);
        p += 4;
        for (var x : triangles) {
            p = x.write(p);
        }
        memPutInt(p, boxes.length);
        p += 4;
        for (var x : boxes) {
            p = x.write(p);
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
