package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
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
    public String label = "";

    int encodedSize() {
        return 40 + label.getBytes(StandardCharsets.UTF_8).length;
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
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
