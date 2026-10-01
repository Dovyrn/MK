package dev.dov.mk;

import lombok.experimental.UtilityClass;

@UtilityClass
public class Compare {
    public final int NONE = 0;
    public final int NEVER = 1;
    public final int LESS = 2;
    public final int EQUAL = 3;
    public final int LESS_EQUAL = 4;
    public final int GREATER = 5;
    public final int NOT_EQUAL = 6;
    public final int GREATER_EQUAL = 7;
    public final int ALWAYS = 8;
}
