#include "mk.slangh"

struct Push {
    float* out;
    DescriptorHandle<RaytracingAccelerationStructure> scene;
    uint size;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

[shader("compute")]
[numthreads(8, 8, 1)]
void trace(uint3 id: SV_DispatchThreadID) {
    RayDesc ray;
    ray.Origin = float3((float2(id.xy) + 0.5) / push.size * 2 - 1, -1);
    ray.Direction = float3(0, 0, 1);
    ray.TMin = 0;
    ray.TMax = 10;
    RayQuery<RAY_FLAG_NONE> q;
    q.TraceRayInline(push.scene, RAY_FLAG_NONE, 0xff, ray);
    q.Proceed();
    float t = q.CommittedStatus() == COMMITTED_TRIANGLE_HIT ? q.CommittedRayT() : -1;
    push.out[id.y * push.size + id.x] = t;
}
