#include "mk.slangh"

BUFFER(0) RWStructuredBuffer<uint> results;

groupshared uint seen[64];

[shader("compute")]
[numthreads(64, 1, 1)]
void lanes(uint i: SV_GroupIndex) {
    uint lane = WaveGetLaneIndex();
    uint4 ballot = WaveActiveBallot(lane % 2 == 0);
    uint at = i * 11;
    results[at + 0] = WaveGetLaneCount();
    results[at + 1] = lane;
    results[at + 2] = WaveActiveSum(1u);
    results[at + 3] = WavePrefixSum(lane);
    results[at + 4] = ballot.x;
    results[at + 5] = WaveReadLaneFirst(lane + 5);
    results[at + 6] = WaveReadLaneAt(lane, 1);
    results[at + 7] = QuadReadAcrossX(lane);
    results[at + 8] = WaveActiveAllTrue(lane < 1000) ? 1 : 0;
    results[at + 9] = WaveActiveMax(lane);
    seen[i] = lane;
    GroupMemoryBarrierWithGroupSync();
    results[at + 10] = seen[63 - i];
}
