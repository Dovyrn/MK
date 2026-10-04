package dev.dov.mk;

public interface Subgroup {
    int BASIC = 1;
    int VOTE = 2;
    int ARITHMETIC = 4;
    int BALLOT = 8;
    int SHUFFLE = 16;
    int RELATIVE = 32;
    int CLUSTERED = 64;
    int QUAD = 128;
    int ROTATE = 512;
}
