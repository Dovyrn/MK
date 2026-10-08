package dev.dov.mk;

public interface Stage {
    int VERTEX = 1;
    int FRAGMENT = 2;
    int TASK = 8;
    int MESH = 16;
    int INDIRECT = 32;
    int DEPTH = 64;
    int COLOR = 128;
    int HOST = 256;
    int COMPUTE = 134217728;
    int COPY = 268435456;
}
