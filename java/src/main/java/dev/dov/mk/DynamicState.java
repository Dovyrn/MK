package dev.dov.mk;

public interface DynamicState {
    int CULL = 1;
    int FACE = 2;
    int DEPTH_STATE = 4;
    int STENCIL_REFERENCE = 8;
    int DEPTH_BIAS = 16;
    int BLEND_COLOR = 32;
    int TOPOLOGY = 64;
}
