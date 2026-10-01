#include <metal_stdlib>
using namespace metal;

struct Push {
    command_buffer commands;
    uint count;
};

kernel void encode(uint id [[thread_position_in_grid]], constant Push& push [[buffer(0)]]) {
    if (id >= push.count) {
        return;
    }
    render_command command(push.commands, id);
    command.draw_primitives(primitive_type::triangle, 0, 3, 1, 0);
}

struct DispatchPush {
    command_buffer commands;
    uint groups;
};

kernel void encode_dispatch(uint id [[thread_position_in_grid]], constant DispatchPush& push [[buffer(0)]]) {
    if (id != 0) {
        return;
    }
    compute_command command(push.commands, 0);
    command.concurrent_dispatch_threadgroups(uint3(push.groups, 1, 1), uint3(64, 1, 1));
}
