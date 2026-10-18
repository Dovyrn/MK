#include "mk.slangh"

struct Push {
    uint* data;
    uint count;
};

BUFFER(0) Uniform<Push> push;

[shader("compute")]
[numthreads(64, 1, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    if (id.x < push.count) {
        push.data[id.x] = id.x * 3 + 7;
    }
}
