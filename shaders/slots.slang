#include "mk.slangh"

struct Push {
    uint count;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

struct Params {
    float4 tint;
};

ConstantBuffer<Params> params : UNIFORM(0);
RWStructuredBuffer<uint> data : STORAGE(0);
RWTexture2D<float4> image : IMAGE(0);
Texture2D<float4> source : TEXTURE(0);
SamplerState nearest : SAMPLER(0);

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
Varyings vert(uint id: SV_VertexID) {
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
