package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Stencil {
    public final int KEEP = 0;
    public final int ZERO = 1;
    public final int REPLACE = 2;
    public final int INCREMENT_CLAMP = 3;
    public final int DECREMENT_CLAMP = 4;
    public final int INVERT = 5;
    public final int INCREMENT_WRAP = 6;
    public final int DECREMENT_WRAP = 7;
}
