#include "mk.slangh"

struct Push {
    uint2* range;
    IndirectBuffer list;
};

Uniform<Push> push : UNIFORM(0);

[shader("compute")]
[numthreads(1, 1, 1)]
void encode() {
    push.list.draw(0, 0, 1, 0, 0);
    push.list.draw(1, 3, 1, 0, 0);
    write_indirect_count(push.range, 2);
}
