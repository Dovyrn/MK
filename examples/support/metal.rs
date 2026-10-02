use mk::objc2_metal::MTLCreateSystemDefaultDevice;

pub fn device(queues: usize) -> mk::Device {
    let raw = MTLCreateSystemDefaultDevice().expect("no Metal device");
    mk::Device::adopt(raw, queues).unwrap()
}

pub fn try_device(queues: usize) -> Option<mk::Device> {
    Some(device(queues))
}
