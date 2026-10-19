#include "mk.slangh"

TEXTURE(0) RWTexture2D<uint64_t> peaks;

[shader("compute")]
[numthreads(64, 1, 1)]
void wide(uint i: SV_GroupIndex) {
    atomic_max(peaks, uint2(i % 4, 0), (uint64_t(i) << 40) | i);
}
