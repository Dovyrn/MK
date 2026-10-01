#include <metal_stdlib>
using namespace metal;

struct Pixel {
    half4 color [[color(0)]];
};

kernel void invert(imageblock<Pixel, imageblock_layout_implicit> block, ushort2 id [[thread_position_in_threadgroup]]) {
    Pixel p = block.read(id);
    p.color = half4(1.0h - p.color.rgb, p.color.a);
    block.write(p, id);
}
