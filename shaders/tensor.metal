#include <metal_stdlib>
#include <metal_tensor>
#include <MetalPerformancePrimitives/MetalPerformancePrimitives.h>
using namespace metal;
using namespace mpp::tensor_ops;

struct Push {
    tensor<device float, dextents<int32_t, 2>, tensor_handle> c;
};

kernel void multiply(
    constant Push& push [[buffer(0)]],
    tensor<device half, dextents<int32_t, 2>> a [[buffer(5)]],
    tensor<device half, dextents<int32_t, 2>> b [[buffer(6)]],
    uint2 group [[threadgroup_position_in_grid]]
) {
    constexpr auto desc = matmul2d_descriptor(32, 32, static_cast<int>(dynamic_extent));
    matmul2d<desc, execution_simdgroups<4>> op;
    auto left = a.slice(0, group.y * 32);
    auto right = b.slice(group.x * 32, 0);
    auto sum = push.c.slice(group.x * 32, group.y * 32);
    op.run(left, right, sum);
}
