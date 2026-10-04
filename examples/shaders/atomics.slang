#include "mk.slangh"

RWStructuredBuffer<uint64_t> results : STORAGE(0);

[shader("compute")]
[numthreads(64, 1, 1)]
void extremes(uint i: SV_GroupIndex) {
    uint64_t value = (uint64_t(i) << 33) | (63 - i);
    atomic_max(results, 0, value);
    atomic_min(results, 1, value);
}

[shader("compute")]
[numthreads(64, 1, 1)]
void sums(uint i: SV_GroupIndex) {
    uint64_t before;
    InterlockedAdd(results[0], uint64_t(1) << 32, before);
    InterlockedOr(results[1], uint64_t(1) << (i % 64), before);
}
