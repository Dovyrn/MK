#include "mk.slangh"

struct Push {
    uint2* range;
    MeshIndirectBuffer list;
};

BUFFER(0) Uniform<Push> push;

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 1, 1, 1);
    write_indirect_count(push.range, 1);
}
