#include "mk.slangh"

struct Push {
    DescriptorHandle<RWStructuredBuffer<uint2>> range;
    MeshIndirectBuffer list;
};

Uniform<Push> push : UNIFORM(0);

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 1, 1, 1);
    push.range[0] = uint2(0, 1);
}
