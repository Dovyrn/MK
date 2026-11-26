package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class IndirectBufferInfo {
    public int maxDraws = 0;
    public Memory memory = Memory.DEVICE;
    public RenderPipeline pipeline = null;
    public boolean indexed = false;
    public Buffer indexBuffer = null;
    public long indexOffset = 0;
    public IndexType indexType = IndexType.U16;
    public String label = "";

    int encodedSize() {
        return 44 + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutInt(p, maxDraws);
        p += 4;
        memPutInt(p, memory.value);
        p += 4;
        memPutLong(p, RenderPipeline.raw(pipeline));
        p += 8;
        memPutInt(p, indexed ? 1 : 0);
        p += 4;
        memPutLong(p, Buffer.raw(indexBuffer));
        p += 8;
        memPutLong(p, indexOffset);
        p += 8;
        memPutInt(p, indexType.value);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
