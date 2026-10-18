package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class DepthStateInfo {
    public CompareOp compare = CompareOp.NONE;
    public boolean write = false;
    public boolean stencil = false;
    public StencilFace front = new StencilFace();
    public StencilFace back = new StencilFace();
    public String label = "";

    int encodedSize() {
        return 16 + front.encodedSize() + back.encodedSize() + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutInt(p, compare.value);
        p += 4;
        memPutInt(p, write ? 1 : 0);
        p += 4;
        memPutInt(p, stencil ? 1 : 0);
        p += 4;
        p = front.encode(p);
        p = back.encode(p);
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
