use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTL4CommandEncoder, MTL4ComputeCommandEncoder,
    MTL4VisibilityOptions, MTLComputePipelineState, MTLDevice, MTLLibrary, MTLSize, MTLStages,
};

const SOURCE: &str = "#include <metal_stdlib>
using namespace metal;
struct Draw {
    device const uchar* args;
    device const uint* count;
    device uint* range;
    device uchar* index;
    uint stride;
    uint max;
    uint base;
    uint primitive;
    uint size;
};
struct Head {
    command_buffer commands;
    uint draws;
};
kernel void encode(constant Head& head [[buffer(0)]],
                   device const Draw* draws [[buffer(1)]],
                   uint id [[thread_position_in_grid]]) {
    uint j = 0;
    while (j < head.draws && id >= draws[j].base + draws[j].max) {
        j++;
    }
    if (j == head.draws) {
        return;
    }
    Draw d = draws[j];
    uint i = id - d.base;
    uint n = min(*d.count, d.max);
    if (i == 0) {
        d.range[0] = d.base;
        d.range[1] = n;
    }
    if (i >= n) {
        return;
    }
    render_command c(head.commands, id);
    device const uint* a = (device const uint*)(d.args + i * d.stride);
    primitive_type p = static_cast<primitive_type>(d.primitive);
    if (a[0] == 0 || a[1] == 0) {
        c.reset();
    } else if (d.size == 0) {
        c.draw_primitives(p, a[2], a[0], a[1], a[3]);
    } else if (d.size == 2) {
        c.draw_indexed_primitives(p, a[0], (device ushort*)d.index + a[2], a[1], a[3], a[4]);
    } else {
        c.draw_indexed_primitives(p, a[0], (device uint*)d.index + a[2], a[1], a[3], a[4]);
    }
}
";

pub struct Count {
    kernel: Retained<ProtocolObject<dyn MTLComputePipelineState>>,
    table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
}

impl Count {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(2);
        let table = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SOURCE), None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let function = library
            .newFunctionWithName(&NSString::from_str("encode"))
            .ok_or("count kernel allocation failed")?;
        let kernel = device
            .newComputePipelineStateWithFunction_error(&function)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(Self { kernel, table })
    }

    pub fn encode(
        &self,
        encoder: &ProtocolObject<dyn MTL4ComputeCommandEncoder>,
        head: u64,
        draws: u64,
        threads: u32,
        render: MTLStages,
    ) {
        unsafe {
            self.table.setAddress_atIndex(head, 0);
            self.table.setAddress_atIndex(draws, 1);
        }
        encoder.setArgumentTable(Some(&self.table));
        encoder.setComputePipelineState(&self.kernel);
        encoder.dispatchThreads_threadsPerThreadgroup(
            MTLSize {
                width: threads as usize,
                height: 1,
                depth: 1,
            },
            MTLSize {
                width: 64,
                height: 1,
                depth: 1,
            },
        );
        encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(
            MTLStages::Dispatch,
            render,
            MTL4VisibilityOptions::None,
        );
    }
}
