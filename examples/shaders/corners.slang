struct Varyings {
    float4 position : SV_Position;
    float3 color : COLOR;
};

[shader("vertex")]
Varyings vert(uint vertex : SV_VulkanVertexID) {
    float2 corners[3] = { float2(-0.5, -0.5), float2(0.5, -0.5), float2(0, 0.5) };
    Varyings o;
    o.position = float4(corners[vertex], 0.5, 1);
    o.color = float3(1, 0, 0);
    return o;
}

[shader("fragment")]
float4 frag(Varyings v) : SV_Target {
    return float4(v.color, 1);
}
