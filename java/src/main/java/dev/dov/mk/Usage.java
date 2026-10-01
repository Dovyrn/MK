package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Usage {
    public final int SAMPLED = 1;
    public final int STORAGE = 2;
    public final int COLOR = 4;
    public final int DEPTH = 8;
    public final int COPY_SRC = 16;
    public final int COPY_DST = 32;
    public final int VIEW = 64;
}
