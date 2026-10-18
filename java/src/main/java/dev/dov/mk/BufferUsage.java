package dev.dov.mk;

public interface BufferUsage {
    int UNIFORM = 1;
    int STORAGE = 2;
    int VERTEX = 4;
    int INDEX = 8;
    int INDIRECT = 16;
    int COPY_SRC = 32;
    int COPY_DST = 64;
    int ADDRESS = 128;
}
