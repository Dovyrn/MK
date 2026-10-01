#include <metal_stdlib>
#include <metal_raytracing>
using namespace metal;
using namespace raytracing;

struct BoxResult {
    bool accept [[accept_intersection]];
    float distance [[distance]];
};

[[intersection(bounding_box, instancing)]]
BoxResult box(float3 origin [[origin]], float3 direction [[direction]], float min_distance [[min_distance]], float max_distance [[max_distance]]) {
    float t = -origin.z / direction.z;
    BoxResult r;
    r.accept = t >= min_distance && t <= max_distance;
    r.distance = t;
    return r;
}

struct Push {
    device float* out;
    instance_acceleration_structure scene;
    intersection_function_table<instancing> functions;
    uint size;
};

kernel void trace(uint2 id [[thread_position_in_grid]], constant Push& push [[buffer(0)]]) {
    ray r;
    r.origin = float3((float2(id) + 0.5) / float(push.size) * 2 - 1, -1);
    r.direction = float3(0, 0, 1);
    r.min_distance = 0;
    r.max_distance = 10;
    intersector<instancing> i;
    auto hit = i.intersect(r, push.scene, 0xff, push.functions);
    push.out[id.y * push.size + id.x] = hit.type == intersection_type::bounding_box ? hit.distance : -1;
}
