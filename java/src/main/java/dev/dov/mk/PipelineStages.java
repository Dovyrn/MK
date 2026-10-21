package dev.dov.mk;

public interface PipelineStages {
    int VERTEX = 1;
    int FRAGMENT = 2;
    int OBJECT = 8;
    int MESH = 16;
    int INDIRECT = 32;
    int DEPTH_STENCIL = 64;
    int COLOR_ATTACHMENT = 128;
    int HOST = 256;
    int COMPUTE = 134217728;
    int COPY = 268435456;
}
