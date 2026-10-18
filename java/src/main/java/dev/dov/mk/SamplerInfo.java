package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class SamplerInfo {
    public Filter min = Filter.LINEAR;
    public Filter mag = Filter.LINEAR;
    public Filter mip = Filter.LINEAR;
    public AddressMode u = AddressMode.REPEAT;
    public AddressMode v = AddressMode.REPEAT;
    public AddressMode w = AddressMode.REPEAT;
    public int anisotropy = 1;
    public CompareOp compare = CompareOp.NONE;
    public float minLod = 0.0f;
    public float maxLod = 1000.0f;
    public BorderColor border = BorderColor.TRANSPARENT_BLACK;
    public String label = "";

    int encodedSize() {
        return 48 + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutInt(p, min.value);
        p += 4;
        memPutInt(p, mag.value);
        p += 4;
        memPutInt(p, mip.value);
        p += 4;
        memPutInt(p, u.value);
        p += 4;
        memPutInt(p, v.value);
        p += 4;
        memPutInt(p, w.value);
        p += 4;
        memPutInt(p, anisotropy);
        p += 4;
        memPutInt(p, compare.value);
        p += 4;
        memPutFloat(p, minLod);
        p += 4;
        memPutFloat(p, maxLod);
        p += 4;
        memPutInt(p, border.value);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
