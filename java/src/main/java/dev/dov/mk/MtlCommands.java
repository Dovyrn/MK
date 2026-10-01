package dev.dov.mk;

import static org.lwjgl.system.MemoryUtil.*;

public class MtlCommands extends Commands {
    public void dispatchThreads(int x, int y, int z) {
        var p = head(11, 12);
        memPutInt(p, x);
        memPutInt(p + 4, y);
        memPutInt(p + 8, z);
    }
}
