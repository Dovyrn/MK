#include "mk.slangh"

struct Camera {
    float4 clip[4];
    float4 model[3];
};

BUFFER(0) Uniform<Camera> camera;
TEXTURE(0) Texture2D<float4> albedo;
SAMPLER(0) SamplerState linear;

struct Vertex {
    [[vk::location(0)]] float3 position : POSITION;
    [[vk::location(1)]] float3 normal : NORMAL;
    [[vk::location(2)]] float2 uv : TEXCOORD0;
};

struct Varyings {
    float4 position : SV_Position;
    float3 normal : NORMAL;
    float2 uv : TEXCOORD0;
};

[shader("vertex")]
Varyings vert(Vertex v) {
    float4 p = float4(v.position, 1);
    Varyings o;
    o.position = float4(dot(camera.clip[0], p), dot(camera.clip[1], p), dot(camera.clip[2], p), dot(camera.clip[3], p));
    o.normal = float3(dot(camera.model[0].xyz, v.normal), dot(camera.model[1].xyz, v.normal), dot(camera.model[2].xyz, v.normal));
    o.uv = v.uv;
    return o;
}

[shader("fragment")]
float4 frag(Varyings v) : SV_Target {
    float3 light = normalize(float3(0.4, 0.8, 0.5));
    float4 color = albedo.Sample(linear, v.uv);
    if (color.a < 0.5) {
        discard;
    }
    float diffuse = abs(dot(normalize(v.normal), light));
    return float4(color.rgb * (0.4 + 0.6 * diffuse), 1);
}
