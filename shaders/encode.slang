#include "mk.slangh"

struct Push {
    DescriptorHandle<RWStructuredBuffer<uint2>> range;
    IndirectBuffer list;
    IndexedIndirectBuffer indexed;
    uint count;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 3, 1, 0, 0);
    push.indexed.draw(0, 3, 1, 1, 0, 0);
    push.range[0] = uint2(0, push.count);
}
