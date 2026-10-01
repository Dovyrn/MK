struct Push {
    float* out;
    DescriptorHandle<RaytracingAccelerationStructure> scene;
    uint size;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

struct Payload {
    float t;
};

struct Attributes {
    float2 uv;
};

[shader("raygeneration")]
void raygen() {
    uint2 id = DispatchRaysIndex().xy;
    RayDesc ray;
    ray.Origin = float3((float2(id) + 0.5) / push.size * 2 - 1, -1);
    ray.Direction = float3(0, 0, 1);
    ray.TMin = 0;
    ray.TMax = 10;
    Payload p;
    p.t = -1;
    TraceRay(push.scene, RAY_FLAG_NONE, 0xff, 0, 0, 0, ray, p);
    push.out[id.y * push.size + id.x] = p.t;
}

[shader("miss")]
void miss(inout Payload p) {
    p.t = -1;
}

[shader("intersection")]
void box() {
    float t = -ObjectRayOrigin().z / ObjectRayDirection().z;
    Attributes a;
    a.uv = float2(0, 0);
    ReportHit(t, 0, a);
}

[shader("closesthit")]
void hit(inout Payload p, Attributes a) {
    p.t = RayTCurrent();
}
