#include "mk.slangh"

TEXTURE(0) RWTexture2D<uint> counts;

[shader("compute")]
[numthreads(64, 1, 1)]
void texels(uint i: SV_GroupIndex) {
    InterlockedAdd(counts[uint2(i % 4, i / 4 % 4)], i);
}
