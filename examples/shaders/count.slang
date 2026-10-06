#include "mk.slangh"

struct Push {
    uint count;
    uint value;
};

Uniform<Push> push : UNIFORM(0);

RWStructuredBuffer<uint> data : STORAGE(0);

[shader("compute")]
[numthreads(64, 1, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    if (id.x < push.count) {
        data[id.x] = push.value + id.x;
    }
}
