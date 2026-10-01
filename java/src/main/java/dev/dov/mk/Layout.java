package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Layout {
    public final int UNDEFINED = 0;
    public final int GENERAL = 1;
    public final int READ = 2;
    public final int COLOR = 3;
    public final int DEPTH = 4;
    public final int DEPTH_READ = 5;
    public final int COPY_SRC = 6;
    public final int COPY_DST = 7;
    public final int PRESENT = 8;
}
