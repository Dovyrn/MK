package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

import java.nio.charset.StandardCharsets;
import lombok.Setter;
import lombok.experimental.Accessors;

@Setter
@Accessors(fluent = true, chain = true)
public class TextureInfo {
    public TextureDimension dimension = TextureDimension.D2;
    public Format format = Format.NONE;
    public int width = 1;
    public int height = 1;
    public int depth = 1;
    public int layerCount = 1;
    public int mipCount = 1;
    public int sampleCount = 1;
    public int usage = 0;
    public Memory memory = Memory.DEVICE;
    public String label = "";

    int encodedSize() {
        return 44 + label.getBytes(StandardCharsets.UTF_8).length;
    }

    long encode(long p) {
        memPutInt(p, dimension.value);
        p += 4;
        memPutInt(p, format.value);
        p += 4;
        memPutInt(p, width);
        p += 4;
        memPutInt(p, height);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, layerCount);
        p += 4;
        memPutInt(p, mipCount);
        p += 4;
        memPutInt(p, sampleCount);
        p += 4;
        memPutInt(p, usage);
        p += 4;
        memPutInt(p, memory.value);
        p += 4;
        var labelBytes = label.getBytes(StandardCharsets.UTF_8);
        memPutInt(p, labelBytes.length);
        memByteBuffer(p + 4, labelBytes.length).put(labelBytes);
        p += 4 + labelBytes.length;
        return p;
    }
}
