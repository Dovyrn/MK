#include "mk.slangh"

struct Push {
    uint2* range;
    MeshIndirectBuffer list;
};

Uniform<Push> push : UNIFORM(0);

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 1, 1, 1);
    write_indirect_count(push.range, 1);
}
