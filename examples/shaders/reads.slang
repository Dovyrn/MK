#include "mk.slangh"

struct Push {
    float value;
};

BUFFER(0) Uniform<Push> push;

INPUT(0) TEXTURE(0) SubpassInput<float4> value;

struct Targets {
    float4 value : SV_Target0;
    float4 color : SV_Target1;
};

[shader("vertex")]
float4 cover(uint v: SV_VulkanVertexID) : SV_Position {
    float2 uv = float2(v << 1 & 2, v & 2);
    return float4(uv * 2 - 1, 0, 1);
}

[shader("fragment")]
Targets fill(float4 p: SV_Position) {
    return { float4(push.value), float4(0) };
}

[shader("fragment")]
Targets add(float4 p: SV_Position) {
    return { value.SubpassLoad() + push.value, float4(0) };
}

[shader("fragment")]
Targets copy(float4 p: SV_Position) {
    float4 v = value.SubpassLoad();
    return { v, float4(v.x, 0, 0, 1) };
}
