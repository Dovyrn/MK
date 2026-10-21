package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SamplerInfo {
    public Filter minFilter = Filter.LINEAR;
    public Filter magFilter = Filter.LINEAR;
    public Filter mipFilter = Filter.LINEAR;
    public AddressMode u = AddressMode.REPEAT;
    public AddressMode v = AddressMode.REPEAT;
    public AddressMode w = AddressMode.REPEAT;
    public int maxAnisotropy = 1;
    public CompareOp compare = CompareOp.NONE;
    public float minLod = 0.0f;
    public float maxLod = 1000.0f;
    public BorderColor borderColor = BorderColor.TRANSPARENT_BLACK;
    public String label = "";

    int encodedSize() {
        return 48 + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutInt(p, minFilter.value);
        p += 4;
        memPutInt(p, magFilter.value);
        p += 4;
        memPutInt(p, mipFilter.value);
        p += 4;
        memPutInt(p, u.value);
        p += 4;
        memPutInt(p, v.value);
        p += 4;
        memPutInt(p, w.value);
        p += 4;
        memPutInt(p, maxAnisotropy);
        p += 4;
        memPutInt(p, compare.value);
        p += 4;
        memPutFloat(p, minLod);
        p += 4;
        memPutFloat(p, maxLod);
        p += 4;
        memPutInt(p, borderColor.value);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
