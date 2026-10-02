package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Stage {
    public final int VERTEX = 1;
    public final int FRAGMENT = 2;
    public final int TILE = 4;
    public final int TASK = 8;
    public final int MESH = 16;
    public final int RAY = 512;
    public final int INDIRECT = 32;
    public final int DEPTH = 64;
    public final int COLOR = 128;
    public final int HOST = 256;
    public final int RESOURCE = 67108864;
    public final int COMPUTE = 134217728;
    public final int COPY = 268435456;
    public final int ACCEL = 536870912;
    public final int ML = 1073741824;
}
