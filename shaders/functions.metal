#include <metal_stdlib>
using namespace metal;

[[visible]] uint add(uint x) {
    return x + 100;
}

[[visible]] uint twice(uint x) {
    return x * 2;
}

struct Push {
    device uint* out;
    visible_function_table<uint(uint)> table;
    uint count;
};

kernel void call(uint id [[thread_position_in_grid]], constant Push& push [[buffer(0)]]) {
    if (id >= push.count) {
        return;
    }
    push.out[id] = push.table[id % 2](id);
}
