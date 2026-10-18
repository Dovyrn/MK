#include "mk.slangh"

struct Push {
    DescriptorHandle<RWTexture2D<float4>> image;
    uint size;
};

BUFFER(0) Uniform<Push> push;

[shader("compute")]
[numthreads(8, 8, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    float4 c = push.image[id.xy];
    push.image[id.xy] = float4(float2(id.xy) / float(push.size), c.z + 0.5, 1);
}
