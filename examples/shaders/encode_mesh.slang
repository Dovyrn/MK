#include "mk.slangh"

struct Push {
    DescriptorHandle<RWStructuredBuffer<uint2>> range;
    MeshIndirectBuffer list;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 1, 1, 1);
    push.range[0] = uint2(0, 1);
}
