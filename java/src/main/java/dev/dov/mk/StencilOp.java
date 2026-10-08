package dev.dov.mk;

public interface StencilOp {
    int KEEP = 0;
    int ZERO = 1;
    int REPLACE = 2;
    int INCREMENT_CLAMP = 3;
    int DECREMENT_CLAMP = 4;
    int INVERT = 5;
    int INCREMENT_WRAP = 6;
    int DECREMENT_WRAP = 7;
}
