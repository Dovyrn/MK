[shader("vertex")]
float4 vert(uint vertex : SV_VulkanVertexID) : SV_Position {
    float2 corners[3] = { float2(-0.5, -0.5), float2(0.5, -0.5), float2(0, 0.5) };
    return float4(corners[vertex], 0.5, 1);
}

[shader("fragment")]
float4 frag() : SV_Target {
    return float4(1, 0, 0, 1);
}
