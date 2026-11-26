use super::{Device, id, labels};
use crate::Error;
use crate::indirect::indirect_stride;
use crate::mtl::indirect::Indirect;
use crate::{IndexType, IndirectBuffer, IndirectBufferInfo, Memory};
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLDevice, MTLIndexType, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLIndirectRenderCommand, MTLResource, MTLResourceOptions, MTLSize,
};

impl Device {
    pub fn create_indirect_buffer(
        &self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, Error> {
        let entry = self.res.table.check_indirect(info);
        let shader = info.memory == Memory::Device;
        self.caps
            .check_indirect(entry.mesh, shader, info.max_draws)?;
        let storage = if shader {
            MTLResourceOptions::StorageModePrivate
        } else {
            MTLResourceOptions::StorageModeShared
        };
        let render = self.res.renders.get(info.pipeline);
        let (task, mesh) = (render.task, render.mesh);
        let groups = [
            task.width,
            task.height,
            task.depth,
            mesh.width,
            mesh.height,
            mesh.depth,
        ]
        .map(|n| n as u32);
        autoreleasepool(|_| {
            let desc = MTLIndirectCommandBufferDescriptor::new();
            desc.setCommandTypes(match (entry.mesh, info.indexed) {
                (true, _) => MTLIndirectCommandType::DrawMeshThreadgroups,
                (_, true) => MTLIndirectCommandType::DrawIndexed,
                _ => MTLIndirectCommandType::Draw,
            });
            desc.setInheritPipelineState(true);
            desc.setInheritBuffers(true);
            let raw = unsafe {
                self.raw
                    .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                        &desc,
                        info.max_draws as usize,
                        storage | MTLResourceOptions::HazardTrackingModeUntracked,
                    )
                    .ok_or_else(|| {
                        Error::OutOfMemory("indirect command buffer allocation failed".into())
                    })?
            };
            if let Some(text) = labels::label(info.label) {
                raw.setLabel(Some(&text));
            }
            let id = self.res.indirects.insert(Indirect { raw, groups });
            self.res.table.indirects.set(id, entry);
            Ok(id)
        })
    }

    pub fn destroy_indirect_buffer(&self, indirect: IndirectBuffer) {
        self.res.table.drop_indirect(indirect);
        self.res.indirects.remove(indirect);
    }

    pub fn write_indirect_draws(&self, indirect: IndirectBuffer, first: u32, draws: &[u8]) {
        self.res
            .table
            .check_indirect_write(indirect, first, draws.len());
        autoreleasepool(|_| {
            self.encode_draws(indirect, first, draws);
        });
    }

    fn encode_draws(&self, indirect: IndirectBuffer, first: u32, draws: &[u8]) {
        let e = *self.res.table.indirects.get(indirect);
        let list = self.res.indirects.get(indirect);
        let render = self.res.renders.get(e.info.pipeline);
        let stride = indirect_stride(e.mesh, e.info.indexed) as usize;
        let (kind, size) = match e.info.index_type {
            IndexType::U16 => (MTLIndexType::UInt16, 2),
            _ => (MTLIndexType::UInt32, 4),
        };
        for (i, draw) in draws.chunks_exact(stride).enumerate() {
            let word = |n: usize| u32::from_ne_bytes(draw[n * 4..n * 4 + 4].try_into().unwrap());
            let at = |n: usize| word(n) as usize;
            unsafe {
                let command = list.raw.indirectRenderCommandAtIndex(first as usize + i);
                if e.mesh {
                    let groups = MTLSize {
                        width: at(0),
                        height: at(1),
                        depth: at(2),
                    };
                    command
                        .drawMeshThreadgroups_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                            groups,
                            render.task,
                            render.mesh,
                        );
                } else if e.info.indexed {
                    let indices = &self.res.buffers.get(e.info.index_buffer).raw;
                    command.drawIndexedPrimitives_indexCount_indexType_indexBuffer_indexBufferOffset_instanceCount_baseVertex_baseInstance(
                        render.primitive,
                        at(0),
                        kind,
                        indices,
                        e.info.index_offset as usize + at(2) * size,
                        at(1),
                        word(3) as i32 as isize,
                        at(4),
                    );
                } else {
                    command.drawPrimitives_vertexStart_vertexCount_instanceCount_baseInstance(
                        render.primitive,
                        at(2),
                        at(0),
                        at(1),
                        at(3),
                    );
                }
            }
        }
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        let info = self.res.table.indirects.get(indirect).info;
        let list = self.res.indirects.get(indirect);
        let indices = if info.indexed {
            self.res.buffers.get(info.index_buffer).address + info.index_offset
        } else {
            0
        };
        let commands = id(list.raw.gpuResourceID());
        self.res
            .table
            .shader_handle(indirect, commands, indices, list.groups)
    }
}
