use crate::mtl::texture::Texture;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTL4CommandEncoder, MTL4ComputeCommandEncoder,
    MTL4VisibilityOptions, MTLComputePipelineState, MTLDevice, MTLLibrary, MTLSize, MTLStages,
    MTLTexture,
};

const SOURCE: &str = "#include <metal_stdlib>
using namespace metal;
template <typename T>
void resolve(texture2d_ms<T> src, texture2d<T, access::write> dst, uint2 id) {
    if (id.x < dst.get_width() && id.y < dst.get_height()) {
        dst.write(src.read(id, 0), id);
    }
}
kernel void unsigned_resolve(texture2d_ms<uint> src [[texture(0)]],
                             texture2d<uint, access::write> dst [[texture(1)]],
                             uint2 id [[thread_position_in_grid]]) {
    resolve(src, dst, id);
}
kernel void signed_resolve(texture2d_ms<int> src [[texture(0)]],
                           texture2d<int, access::write> dst [[texture(1)]],
                           uint2 id [[thread_position_in_grid]]) {
    resolve(src, dst, id);
}
";

pub struct Resolve {
    kernels: [Retained<ProtocolObject<dyn MTLComputePipelineState>>; 2],
    table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
}

impl Resolve {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxTextureBindCount(2);
        let table = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SOURCE), None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let kernel = |name: &str| {
            let function = library
                .newFunctionWithName(&NSString::from_str(name))
                .ok_or("resolve kernel allocation failed")?;
            device
                .newComputePipelineStateWithFunction_error(&function)
                .map_err(|e| e.localizedDescription().to_string())
        };
        Ok(Self {
            kernels: [kernel("unsigned_resolve")?, kernel("signed_resolve")?],
            table,
        })
    }

    pub fn encode(
        &self,
        encoder: &ProtocolObject<dyn MTL4ComputeCommandEncoder>,
        pairs: Vec<(&Texture, &Texture, bool)>,
    ) {
        encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(
            MTLStages::Fragment,
            MTLStages::Dispatch,
            MTL4VisibilityOptions::None,
        );
        encoder.setArgumentTable(Some(&self.table));
        for (src, dst, signed) in pairs {
            unsafe {
                self.table.setTexture_atIndex(src.raw.gpuResourceID(), 0);
                self.table.setTexture_atIndex(dst.raw.gpuResourceID(), 1);
            }
            encoder.setComputePipelineState(&self.kernels[signed as usize]);
            encoder.dispatchThreads_threadsPerThreadgroup(
                MTLSize {
                    width: dst.raw.width(),
                    height: dst.raw.height(),
                    depth: 1,
                },
                MTLSize {
                    width: 8,
                    height: 8,
                    depth: 1,
                },
            );
        }
        encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(
            MTLStages::Dispatch,
            MTLStages::All,
            MTL4VisibilityOptions::None,
        );
    }
}
