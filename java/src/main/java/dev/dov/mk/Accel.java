package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Accel {
    public final int UPDATE = 1;
    public final int COMPACT = 2;
    public final int FAST_TRACE = 4;
    public final int FAST_BUILD = 8;
    public final int SMALL = 16;
}
