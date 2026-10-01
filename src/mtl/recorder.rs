use crate::mtl::frame::Frame;
use crate::mtl::resources::Resources;
use crate::{Command, access, format, stage};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSRange, NSString};
use objc2_metal::{
    MTL4ArgumentTable, MTL4CommandBuffer, MTL4CommandEncoder, MTL4ComputeCommandEncoder,
    MTL4VisibilityOptions, MTLBuffer, MTLDevice, MTLOrigin, MTLResidencySet, MTLSize, MTLStages,
};

pub struct Recorder<'a> {
    pub device: &'a ProtocolObject<dyn MTLDevice>,
    pub residency: &'a ProtocolObject<dyn MTLResidencySet>,
    pub buffer: &'a ProtocolObject<dyn MTL4CommandBuffer>,
    pub table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
    pub frame: &'a mut Frame,
    pub res: &'a Resources,
    pub encoder: Option<Retained<ProtocolObject<dyn MTL4ComputeCommandEncoder>>>,
    pub group: MTLSize,
    pub push: [u8; 256],
    pub size: usize,
    pub dirty: bool,
}

const COMPUTE: MTLStages =
    MTLStages(MTLStages::Dispatch.0 | MTLStages::Blit.0 | MTLStages::AccelerationStructure.0);

fn stages(bits: u32) -> MTLStages {
    let mut s = bits & 0x7c00_001f;
    if bits & stage::INDIRECT != 0 {
        s |= stage::VERTEX | stage::TASK | stage::MESH | stage::COMPUTE;
    }
    if bits & (stage::DEPTH | stage::COLOR) != 0 {
        s |= stage::FRAGMENT;
    }
    MTLStages(s as usize)
}

fn visibility(src: u32, dst: u32) -> MTL4VisibilityOptions {
    if (src | dst) & (access::HOST_READ | access::HOST_WRITE) != 0 {
        MTL4VisibilityOptions::Device
    } else {
        MTL4VisibilityOptions::None
    }
}

fn origin(x: u32, y: u32, z: u32) -> MTLOrigin {
    MTLOrigin {
        x: x as usize,
        y: y as usize,
        z: z as usize,
    }
}

fn size(x: u32, y: u32, z: u32) -> MTLSize {
    MTLSize {
        width: x as usize,
        height: y as usize,
        depth: z as usize,
    }
}

fn pitch(format: u32, width: u32, length: u32, height: u32, count: u32) -> (usize, usize) {
    let [bytes, w, h] = format::block(format);
    let length = if length == 0 { width } else { length };
    let count = if count == 0 { height } else { count };
    let row = length.div_ceil(w) * bytes;
    (row as usize, (count.div_ceil(h) * row) as usize)
}

impl Recorder<'_> {
    fn barrier(&self, src_stage: u32, src_access: u32, dst_stage: u32, dst_access: u32) {
        let src = stages(src_stage) & COMPUTE;
        let dst = stages(dst_stage) & COMPUTE;
        if !src.is_empty() && !dst.is_empty() {
            self.encoder()
                .barrierAfterEncoderStages_beforeEncoderStages_visibilityOptions(
                    src,
                    dst,
                    visibility(src_access, dst_access),
                );
        }
    }

    fn encoder(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.encoder.as_deref().unwrap()
    }

    fn flush(&mut self) {
        if self.dirty {
            let address = self
                .frame
                .push(self.device, self.residency, &self.push[..self.size]);
            unsafe { self.table.setAddress_atIndex(address, 0) };
            self.dirty = false;
        }
    }

    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), &'static str> {
        while !bytes.is_empty() {
            self.step(Command::read(&mut bytes)?);
        }
        Ok(())
    }

    fn step(&mut self, command: Command) {
        match command {
            Command::BeginCompute => {
                let encoder = self.buffer.computeCommandEncoder().unwrap();
                encoder.setArgumentTable(Some(self.table));
                self.encoder = Some(encoder);
            }
            Command::EndCompute => self.encoder.take().unwrap().endEncoding(),
            Command::BindPipeline { pipeline } => {
                let p = self.res.pipelines.get(pipeline);
                self.encoder().setComputePipelineState(&p.state);
                self.group = p.group;
            }
            Command::PushConstants { offset, data } => {
                let at = offset as usize;
                self.push[at..at + data.len()].copy_from_slice(data);
                self.size = self.size.max(at + data.len());
                self.dirty = true;
            }
            Command::Dispatch { x, y, z } => {
                self.flush();
                self.encoder()
                    .dispatchThreadgroups_threadsPerThreadgroup(size(x, y, z), self.group);
            }
            Command::DispatchIndirect { buffer, offset } => {
                self.flush();
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                unsafe {
                    self.encoder()
                        .dispatchThreadgroupsWithIndirectBuffer_threadsPerThreadgroup(
                            address, self.group,
                        )
                };
            }
            Command::DispatchThreads { x, y, z } => {
                self.flush();
                self.encoder()
                    .dispatchThreads_threadsPerThreadgroup(size(x, y, z), self.group);
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => unsafe {
                self.encoder()
                    .copyFromBuffer_sourceOffset_toBuffer_destinationOffset_size(
                        self.res.buffers.get(src),
                        src_offset as usize,
                        self.res.buffers.get(dst),
                        dst_offset as usize,
                        size as usize,
                    )
            },
            Command::FillBuffer {
                buffer,
                offset,
                size,
                value,
            } => unsafe {
                self.encoder().fillBuffer_range_value(
                    self.res.buffers.get(buffer),
                    NSRange::new(offset as usize, size as usize),
                    value,
                )
            },
            Command::Barrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            } => self.barrier(src_stage, src_access, dst_stage, dst_access),
            Command::TextureBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
                ..
            } => self.barrier(src_stage, src_access, dst_stage, dst_access),
            Command::CopyBufferToTexture {
                buffer,
                offset,
                row_length,
                row_count,
                texture,
                mip,
                layer,
                x,
                y,
                z,
                width,
                height,
                depth,
                ..
            } => {
                let t = self.res.textures.get(texture);
                let (row, image) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.encoder()
                        .copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                            self.res.buffers.get(buffer),
                            offset as usize,
                            row,
                            image,
                            size(width, height, depth),
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                        )
                }
            }
            Command::CopyTextureToBuffer {
                texture,
                mip,
                layer,
                x,
                y,
                z,
                width,
                height,
                depth,
                buffer,
                offset,
                row_length,
                row_count,
                ..
            } => {
                let t = self.res.textures.get(texture);
                let (row, image) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.encoder()
                        .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                            size(width, height, depth),
                            self.res.buffers.get(buffer),
                            offset as usize,
                            row,
                            image,
                        )
                }
            }
            Command::QueueBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            } => {
                let src = stages(src_stage);
                let dst = stages(dst_stage) & COMPUTE;
                if !src.is_empty() && !dst.is_empty() {
                    self.encoder()
                        .barrierAfterQueueStages_beforeStages_visibilityOptions(
                            src,
                            dst,
                            visibility(src_access, dst_access),
                        );
                }
            }
            Command::BeginLabel { text } => {
                let text = NSString::from_str(text);
                match &self.encoder {
                    Some(e) => e.pushDebugGroup(&text),
                    None => self.buffer.pushDebugGroup(&text),
                }
            }
            Command::EndLabel => match &self.encoder {
                Some(e) => e.popDebugGroup(),
                None => self.buffer.popDebugGroup(),
            },
        }
    }
}
