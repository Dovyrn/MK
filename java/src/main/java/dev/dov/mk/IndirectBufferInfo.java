package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class IndirectBufferInfo {
    public int draws = 0;
    public RenderPipeline pipeline = null;
    public boolean indexed = false;
    public Buffer indices = null;
    public long offset = 0;
    public IndexType kind = IndexType.U16;

    public static final int SIZE = 36;

    int encodedSize() {
        return 36;
    }

    long encode(long p) {
        memPutInt(p, draws);
        p += 4;
        memPutLong(p, RenderPipeline.raw(pipeline));
        p += 8;
        memPutInt(p, indexed ? 1 : 0);
        p += 4;
        memPutLong(p, Buffer.raw(indices));
        p += 8;
        memPutLong(p, offset);
        p += 8;
        memPutInt(p, kind.value);
        p += 4;
        return p;
    }

    static IndirectBufferInfo decode(long p) {
        var r = new IndirectBufferInfo();
        r.draws = memGetInt(p);
        p += 4;
        r.pipeline = RenderPipeline.of(memGetLong(p));
        p += 8;
        r.indexed = memGetInt(p) != 0;
        p += 4;
        r.indices = Buffer.of(memGetLong(p));
        p += 8;
        r.offset = memGetLong(p);
        p += 8;
        r.kind = IndexType.of(memGetInt(p));
        p += 4;
        return r;
    }
}
