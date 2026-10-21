package dev.dov.mk;

public interface DynamicPipelineStates {
    int CULL_MODE = 1;
    int FRONT_FACE = 2;
    int DEPTH_STENCIL_STATE = 4;
    int STENCIL_REFERENCE = 8;
    int DEPTH_BIAS = 16;
    int BLEND_COLOR = 32;
    int PRIMITIVE_TOPOLOGY = 64;
}
