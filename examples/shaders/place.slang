#include "mk.slangh"

struct Push {
    RayInstances instances;
    uint2 accel;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

[shader("compute")]
[numthreads(1, 1, 1)]
void place() {
    float3x4 shifted = float3x4(1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0.5);
    push.instances.write(1, shifted, 1, 0xff, 0, push.accel);
}
