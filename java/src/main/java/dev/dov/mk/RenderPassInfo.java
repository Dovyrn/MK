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
    public DepthAttachment depth = null;
    public QueryPool occlusion = null;
    public TileMemory tile = null;

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 28 + colorsSize + (depth == null ? 0 : depth.encodedSize()) + (tile == null ? 0 : tile.encodedSize());
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
        memPutInt(p, depth == null ? 0 : 1);
        p += 4;
        if (depth != null) {
            p = depth.encode(p);
        }
        memPutLong(p, QueryPool.raw(occlusion));
        p += 8;
        memPutInt(p, tile == null ? 0 : 1);
        p += 4;
        if (tile != null) {
            p = tile.encode(p);
        }
        return p;
    }
}
