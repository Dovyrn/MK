package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class UpscaleInfo {
    public boolean temporal = false;
    public int inputWidth = 1;
    public int inputHeight = 1;
    public int outputWidth = 1;
    public int outputHeight = 1;
    public int color = Format.RGBA16_FLOAT;
    public int output = Format.RGBA16_FLOAT;
    public int depth = Format.D32_FLOAT;
    public int motion = Format.RG16_FLOAT;

    public int size() {
        return 36;
    }

    public long write(long p) {
        memPutInt(p, temporal ? 1 : 0);
        p += 4;
        memPutInt(p, inputWidth);
        p += 4;
        memPutInt(p, inputHeight);
        p += 4;
        memPutInt(p, outputWidth);
        p += 4;
        memPutInt(p, outputHeight);
        p += 4;
        memPutInt(p, color);
        p += 4;
        memPutInt(p, output);
        p += 4;
        memPutInt(p, depth);
        p += 4;
        memPutInt(p, motion);
        p += 4;
        return p;
    }

    public static UpscaleInfo read(long p) {
        var r = new UpscaleInfo();
        r.temporal = memGetInt(p) != 0;
        p += 4;
        r.inputWidth = memGetInt(p);
        p += 4;
        r.inputHeight = memGetInt(p);
        p += 4;
        r.outputWidth = memGetInt(p);
        p += 4;
        r.outputHeight = memGetInt(p);
        p += 4;
        r.color = memGetInt(p);
        p += 4;
        r.output = memGetInt(p);
        p += 4;
        r.depth = memGetInt(p);
        p += 4;
        r.motion = memGetInt(p);
        p += 4;
        return r;
    }
}
