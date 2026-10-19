#include "mk.slangh"

struct Vertex {
    [[vk::location(0)]] float2 position : POSITION;
    [[vk::location(1)]] float3 color : COLOR;
};

struct Varyings {
    float4 position : SV_Position;
    float3 color : COLOR;
};

struct Push {
    float z;
};

BUFFER(0) Uniform<Push> push;

[shader("vertex")]
Varyings vert(Vertex v) {
    Varyings o;
    o.position = float4(v.position, push.z, 1);
    o.color = v.color;
    return o;
}

[shader("fragment")]
float4 frag(Varyings v) : SV_Target {
    return float4(v.color, 1);
}
