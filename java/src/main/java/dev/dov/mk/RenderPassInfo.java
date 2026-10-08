package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class RenderPassInfo {
    public int width = 1;
    public int height = 1;
    public int layers = 1;
    public ColorAttachment[] colors = new ColorAttachment[0];
    public DepthAttachment depth = new DepthAttachment();
    public QueryPool occlusion = null;
    public TileMemory tile = new TileMemory();

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 20 + colorsSize + depth.encodedSize() + tile.encodedSize();
    }

    long encode(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, layers);
        p += 4;
        memPutInt(p, colors.length);
        p += 4;
        for (var x : colors) {
            p = x.encode(p);
        }
        p = depth.encode(p);
        memPutLong(p, QueryPool.raw(occlusion));
        p += 8;
        p = tile.encode(p);
        return p;
    }
}
