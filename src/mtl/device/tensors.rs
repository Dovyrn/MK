use super::{Device, id, options};
use crate::{TensorInfo, element, memory};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, Message};
use objc2_metal::{
    MTLAllocation, MTLBuffer, MTLDevice, MTLTensor, MTLTensorDataType, MTLTensorDescriptor,
    MTLTensorExtents, MTLTensorUsage,
};

fn kind(e: u32) -> Option<(MTLTensorDataType, u64)> {
    Some(match e {
        element::F16 => (MTLTensorDataType::Float16, 2),
        element::F32 => (MTLTensorDataType::Float32, 4),
        element::BF16 => (MTLTensorDataType::BFloat16, 2),
        element::I8 => (MTLTensorDataType::Int8, 1),
        element::U8 => (MTLTensorDataType::UInt8, 1),
        element::I16 => (MTLTensorDataType::Int16, 2),
        element::U16 => (MTLTensorDataType::UInt16, 2),
        element::I32 => (MTLTensorDataType::Int32, 4),
        element::U32 => (MTLTensorDataType::UInt32, 4),
        _ => return None,
    })
}

fn extents(values: &[u32]) -> Retained<MTLTensorExtents> {
    let values: Vec<isize> = values.iter().map(|&v| v as isize).collect();
    unsafe {
        MTLTensorExtents::initWithRank_values(
            MTLTensorExtents::alloc(),
            values.len(),
            values.as_ptr(),
        )
    }
    .expect("checked by Caps::tensor")
}

fn allocation(
    tensor: &ProtocolObject<dyn MTLTensor>,
) -> Retained<ProtocolObject<dyn MTLAllocation>> {
    match tensor.buffer() {
        Some(b) => ProtocolObject::from_retained(b),
        None => ProtocolObject::from_retained(tensor.retain()),
    }
}

impl Device {
    pub fn create_tensor(&mut self, info: &TensorInfo) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.caps.tensor(info)?;
            let (kind, size) = kind(info.element).ok_or(format!(
                "element {} is not a Metal tensor element",
                info.element
            ))?;
            let desc = MTLTensorDescriptor::new();
            desc.setDimensions(&extents(info.dimensions));
            desc.setDataType(kind);
            desc.setUsage(MTLTensorUsage(info.usage as usize));
            desc.setResourceOptions(options(info.memory));
            let raw = if info.memory == memory::DEVICE {
                self.raw.newTensorWithDescriptor_error(&desc)
            } else {
                let mut strides = info.strides.to_vec();
                if strides.is_empty() {
                    strides = info
                        .dimensions
                        .iter()
                        .scan(1, |s, &d| {
                            let stride = *s;
                            *s *= d;
                            Some(stride)
                        })
                        .collect();
                }
                desc.setStrides(Some(&extents(&strides)));
                let last = info.dimensions.len() - 1;
                let bytes = strides[last] as u64 * info.dimensions[last] as u64 * size;
                let buffer = self
                    .raw
                    .newBufferWithLength_options(bytes as usize, options(info.memory))
                    .ok_or("tensor allocation failed")?;
                unsafe { buffer.newTensorWithDescriptor_offset_error(&desc, 0) }
            }
            .map_err(|e| e.localizedDescription().to_string())?;
            self.residency.add(&allocation(&raw));
            let id = self.res.tensors.insert(raw);
            self.res.table.tensors.set(id, info.usage);
            Ok(id)
        })
    }

    pub fn destroy_tensor(&mut self, tensor: u32) {
        self.res.table.tensors.clear(tensor);
        let raw = allocation(&self.res.tensors.remove(tensor));
        self.defer(move |r| r.remove(&raw));
    }

    pub fn tensor_handle(&self, tensor: u32) -> u64 {
        self.res.table.tensors.get(tensor);
        id(self.res.tensors.get(tensor).gpuResourceID())
    }

    pub fn map_tensor(&self, tensor: u32) -> *mut u8 {
        self.res.table.tensors.get(tensor);
        let buffer = self.res.tensors.get(tensor).buffer();
        let buffer = buffer.expect("map of a device tensor");
        buffer.contents().as_ptr().cast()
    }
}
