package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class RenderPassInfo {
    public int width = 1;
    public int height = 1;
    public int layerCount = 1;
    public ColorAttachment[] colors = new ColorAttachment[0];
    public DepthAttachment depthStencil = null;
    public QueryPool occlusionQueries = null;
    public TileInfo tileInfo = null;
    public int srcStage = 0;
    public int dstStage = 0;

    int encodedSize() {
        var colorsSize = 4;
        for (var x : colors) {
            colorsSize += x.encodedSize();
        }
        return 36 + colorsSize + (depthStencil == null ? 0 : depthStencil.encodedSize()) + (tileInfo == null ? 0 : tileInfo.encodedSize());
    }

    long encode(long p) {
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, layerCount);
        p += 4;
        memPutInt(p, colors.length);
        p += 4;
        for (var x : colors) {
            p = x.encode(p);
        }
        memPutInt(p, depthStencil == null ? 0 : 1);
        p += 4;
        if (depthStencil != null) {
            p = depthStencil.encode(p);
        }
        memPutLong(p, QueryPool.raw(occlusionQueries));
        p += 8;
        memPutInt(p, tileInfo == null ? 0 : 1);
        p += 4;
        if (tileInfo != null) {
            p = tileInfo.encode(p);
        }
        memPutInt(p, srcStage);
        p += 4;
        memPutInt(p, dstStage);
        p += 4;
        return p;
    }
}
