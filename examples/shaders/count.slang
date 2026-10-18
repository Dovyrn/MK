#include "mk.slangh"

struct Push {
    uint count;
    uint value;
};

BUFFER(0) Uniform<Push> push;

BUFFER(1) RWStructuredBuffer<uint> data;

[shader("compute")]
[numthreads(64, 1, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    if (id.x < push.count) {
        data[id.x] = push.value + id.x;
    }
}
