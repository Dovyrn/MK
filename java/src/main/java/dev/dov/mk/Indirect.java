package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Indirect {
    public final int DRAW = 1;
    public final int DRAW_INDEXED = 2;
    public final int DISPATCH = 32;
    public final int DISPATCH_THREADS = 64;
    public final int DRAW_MESH = 128;
    public final int DRAW_MESH_THREADS = 256;
}
