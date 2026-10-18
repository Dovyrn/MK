#include "mk.slangh"

struct Push {
    uint count;
};

BUFFER(0) Uniform<Push> push;

struct Params {
    float4 tint;
};

BUFFER(1) Uniform<Params> params;
BUFFER(2) RWStructuredBuffer<uint> data;
TEXTURE(1) RWTexture2D<float4> image;
TEXTURE(0) Texture2D<float4> source;
SAMPLER(0) SamplerState nearest;

[shader("compute")]
[numthreads(64, 1, 1)]
void fill(uint3 id: SV_DispatchThreadID) {
    if (id.x < push.count) {
        float2 uv = float2((id.x % 4 + 0.5) / 4, 0.5);
        float4 c = source.SampleLevel(nearest, uv, 0) * params.tint;
        data[id.x] = uint(round(c.x * 255)) + id.x * 1000;
        image[uint2(id.x % 4, 0)] = c;
    }
}

struct Varyings {
    float4 position : SV_Position;
    float2 uv : TEXCOORD0;
};

[shader("vertex")]
Varyings vert(uint id: SV_VulkanVertexID) {
    float2 p = float2((id << 1) & 2, id & 2);
    Varyings o;
    o.position = float4(p * 2 - 1, 0, 1);
    o.uv = p;
    return o;
}

[shader("fragment")]
float4 frag(Varyings v) : SV_Target {
    return source.Sample(nearest, v.uv) * params.tint;
}
