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
