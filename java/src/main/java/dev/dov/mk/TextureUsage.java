package dev.dov.mk;

public interface TextureUsage {
    int SAMPLED = 1;
    int STORAGE = 2;
    int COLOR_ATTACHMENT = 4;
    int DEPTH_STENCIL = 8;
    int COPY_SRC = 16;
    int COPY_DST = 32;
    int MUTABLE_VIEW_FORMAT = 64;
    int HOST_COPY = 128;
    int STORAGE_ATOMIC = 1024;
    int INPUT_ATTACHMENT = 2048;
}
