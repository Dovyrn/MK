struct Varyings {
    float4 position : SV_Position;
    float3 color : COLOR;
};

struct Payload {
    float scale;
};

struct Push {
    float z;
};

[[vk::push_constant]] ConstantBuffer<Push> push;

[shader("amplification")]
[numthreads(1, 1, 1)]
void task() {
    static groupshared Payload payload;
    payload.scale = 0.5;
    DispatchMesh(1, 1, 1, payload);
}

[shader("mesh")]
[numthreads(3, 1, 1)]
[outputtopology("triangle")]
void mesh(uint i: SV_GroupThreadID, in payload Payload p, out indices uint3 tris[1], out vertices Varyings verts[3]) {
    SetMeshOutputCounts(3, 1);
    float2 corners[3] = { float2(-1, -1), float2(1, -1), float2(0, 1) };
    Varyings v;
    v.position = float4(corners[i] * p.scale, push.z, 1);
    v.color = float3(1, 0, 0);
    verts[i] = v;
    if (i == 0) {
        tris[0] = uint3(0, 1, 2);
    }
}

[shader("mesh")]
[numthreads(3, 1, 1)]
[outputtopology("triangle")]
void single(uint i: SV_GroupThreadID, out indices uint3 tris[1], out vertices Varyings verts[3]) {
    SetMeshOutputCounts(3, 1);
    float2 corners[3] = { float2(-1, -1), float2(1, -1), float2(0, 1) };
    Varyings v;
    v.position = float4(corners[i] * 0.5, push.z, 1);
    v.color = float3(1, 0, 0);
    verts[i] = v;
    if (i == 0) {
        tris[0] = uint3(0, 1, 2);
    }
}

[shader("fragment")]
float4 frag(Varyings v) : SV_Target {
    return float4(v.color, 1);
}
