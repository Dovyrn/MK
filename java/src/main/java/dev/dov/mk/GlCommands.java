package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class GlCommands extends Commands {

    public void setShadingRate(int width, int height, int primitive, int attachment) {
        var p = head(56, 16);
        memPutInt(p, width);
        memPutInt(p + 4, height);
        memPutInt(p + 8, primitive);
        memPutInt(p + 12, attachment);
    }

    public void rateAttachment(int texture, int layout, int texelWidth, int texelHeight) {
        var p = head(57, 16);
        memPutInt(p, texture);
        memPutInt(p + 4, layout);
        memPutInt(p + 8, texelWidth);
        memPutInt(p + 12, texelHeight);
    }
}
