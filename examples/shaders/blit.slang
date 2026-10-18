#include "mk.slangh"

struct Push {
    float4* out;
    DescriptorHandle<Texture2D<float4>> tex;
    DescriptorHandle<SamplerState> samp;
    DescriptorHandle<RWTexture2D<float4>> img;
    uint size;
};

BUFFER(0) Uniform<Push> push;

[shader("compute")]
[numthreads(8, 8, 1)]
void blit(uint3 id: SV_DispatchThreadID) {
    float4 c = push.tex.SampleLevel(push.samp, (float2(id.xy) + 0.5) / float(push.size), 0);
    push.img[id.xy] = c.bgra;
    push.out[id.y * push.size + id.x] = c;
}
