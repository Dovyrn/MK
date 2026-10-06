#include "mk.slangh"

struct Push {
    DescriptorHandle<RWStructuredBuffer<uint>> data;
    uint count;
};

Uniform<Push> push : UNIFORM(0);

[shader("compute")]
[numthreads(64, 1, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    if (id.x < push.count) {
        push.data[id.x] = id.x * 3 + 7;
    }
}
