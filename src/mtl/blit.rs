use crate::{filter, format};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTLDevice, MTLFunction, MTLLibrary,
    MTLPixelFormat, MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSamplerState,
};
use std::cell::RefCell;

const SOURCE: &str = "#include <metal_stdlib>
using namespace metal;
struct Rects {
    float4 src;
    float4 dst;
    float2 size;
    float depth;
};
vertex float4 blit_vertex(uint id [[vertex_id]]) {
    float2 p = float2((id << 1) & 2, id & 2);
    return float4(p * 2.0 - 1.0, 0.0, 1.0);
}
float2 source(float4 position, constant Rects& r) {
    return r.src.xy + (position.xy - r.dst.xy) * r.src.zw / r.dst.zw;
}
template <typename T>
vec<T, 4> fetch(float4 position, texture2d<T> t, constant Rects& r) {
    return t.read(uint2(clamp(source(position, r), float2(0.0), r.size - 1.0)));
}
fragment float4 blit_float(float4 position [[position]],
                           texture2d<float> t [[texture(0)]],
                           sampler s [[sampler(0)]],
                           constant Rects& r [[buffer(0)]]) {
    return t.sample(s, source(position, r) / r.size);
}
fragment float4 blit_volume(float4 position [[position]],
                            texture3d<float> t [[texture(0)]],
                            sampler s [[sampler(0)]],
                            constant Rects& r [[buffer(0)]]) {
    return t.sample(s, float3(source(position, r) / r.size, r.depth));
}
fragment uint4 blit_uint(float4 position [[position]],
                         texture2d<uint> t [[texture(0)]],
                         constant Rects& r [[buffer(0)]]) {
    return fetch(position, t, r);
}
fragment int4 blit_int(float4 position [[position]],
                       texture2d<int> t [[texture(0)]],
                       constant Rects& r [[buffer(0)]]) {
    return fetch(position, t, r);
}
";

type State = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;

pub struct Blit {
    library: Retained<ProtocolObject<dyn MTLLibrary>>,
    samplers: [Retained<ProtocolObject<dyn MTLSamplerState>>; 2],
    pub table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
    states: RefCell<Vec<(MTLPixelFormat, bool, State)>>,
}

fn function(
    library: &ProtocolObject<dyn MTLLibrary>,
    name: &str,
) -> Result<Retained<ProtocolObject<dyn MTLFunction>>, &'static str> {
    library
        .newFunctionWithName(&NSString::from_str(name))
        .ok_or("blit function allocation failed")
}

impl Blit {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(1);
        desc.setMaxTextureBindCount(1);
        desc.setMaxSamplerStateBindCount(1);
        let table = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SOURCE), None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let sampler = |f| {
            let desc = MTLSamplerDescriptor::new();
            desc.setMinFilter(f);
            desc.setMagFilter(f);
            desc.setSupportArgumentBuffers(true);
            device
                .newSamplerStateWithDescriptor(&desc)
                .ok_or("blit sampler allocation failed")
        };
        Ok(Self {
            library,
            samplers: [
                sampler(MTLSamplerMinMagFilter::Nearest)?,
                sampler(MTLSamplerMinMagFilter::Linear)?,
            ],
            table,
            states: RefCell::new(Vec::new()),
        })
    }

    pub fn sampler(&self, f: u32) -> &ProtocolObject<dyn MTLSamplerState> {
        &self.samplers[(f == filter::LINEAR) as usize]
    }

    pub fn state(
        &self,
        device: &ProtocolObject<dyn MTLDevice>,
        target: u32,
        volume: bool,
    ) -> Result<State, &'static str> {
        let pixel = format::mtl(target);
        let mut states = self.states.borrow_mut();
        if let Some((.., s)) = states.iter().find(|(p, v, _)| *p == pixel && *v == volume) {
            return Ok(s.clone());
        }
        let name = if volume {
            "blit_volume"
        } else if format::uint(target) {
            "blit_uint"
        } else if format::sint(target) {
            "blit_int"
        } else {
            "blit_float"
        };
        let desc = MTLRenderPipelineDescriptor::new();
        desc.setVertexFunction(Some(&*function(&self.library, "blit_vertex")?));
        desc.setFragmentFunction(Some(&*function(&self.library, name)?));
        unsafe {
            desc.colorAttachments()
                .objectAtIndexedSubscript(0)
                .setPixelFormat(pixel)
        };
        let state = device
            .newRenderPipelineStateWithDescriptor_error(&desc)
            .map_err(|_| "blit pipeline creation failed")?;
        states.push((pixel, volume, state.clone()));
        Ok(state)
    }
}
