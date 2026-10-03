package dev.dov.mk;

public interface Usage {
    int SAMPLED = 1;
    int STORAGE = 2;
    int COLOR = 4;
    int DEPTH = 8;
    int COPY_SRC = 16;
    int COPY_DST = 32;
    int VIEW = 64;
    int HOST = 128;
}
