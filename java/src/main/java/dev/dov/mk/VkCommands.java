package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class VkCommands extends Commands {
    public void setLineWidth(float width) {
        var p = head(12, 4);
        memPutFloat(p, width);
    }
}
