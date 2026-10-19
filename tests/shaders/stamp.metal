#include <metal_stdlib>
using namespace metal;

struct Pixel {
    half4 color [[color(0)]];
};

kernel void stamp(
    imageblock<Pixel, imageblock_layout_implicit> block,
    threadgroup half* cells [[threadgroup(0)]],
    ushort2 local [[thread_position_in_threadgroup]])
{
    uint i = local.y * 16 + local.x;
    cells[i] = half(i) / 255.0h;
    threadgroup_barrier(mem_flags::mem_threadgroup);
    Pixel p = block.read(local);
    p.color = half4(half(local.x) / 15.0h, half(local.y) / 15.0h, cells[255 - i], p.color.a);
    block.write(p, local);
}
