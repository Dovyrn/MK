package dev.dov.mk;

public interface Stage {
    int VERTEX = 1;
    int FRAGMENT = 2;
    int TILE = 4;
    int TASK = 8;
    int MESH = 16;
    int RAY = 512;
    int INDIRECT = 32;
    int DEPTH = 64;
    int COLOR = 128;
    int HOST = 256;
    int RESOURCE = 67108864;
    int COMPUTE = 134217728;
    int COPY = 268435456;
    int ACCEL = 536870912;
    int ML = 1073741824;
}
