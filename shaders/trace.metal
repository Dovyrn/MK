#include <metal_stdlib>
#include <metal_raytracing>
using namespace metal;
using namespace raytracing;

struct Push {
    device float* out;
    instance_acceleration_structure scene;
    uint size;
};

kernel void trace(uint2 id [[thread_position_in_grid]], constant Push& push [[buffer(0)]]) {
    ray r;
    r.origin = float3((float2(id) + 0.5) / float(push.size) * 2 - 1, -1);
    r.direction = float3(0, 0, 1);
    r.min_distance = 0;
    r.max_distance = 10;
    intersector<triangle_data, instancing> i;
    auto hit = i.intersect(r, push.scene, 0xff);
    push.out[id.y * push.size + id.x] = hit.type == intersection_type::triangle ? hit.distance : -1;
}
