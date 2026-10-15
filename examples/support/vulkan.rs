pub type Host = mk::Device;

pub fn device(queues: u32) -> Host {
    mk::Device::create_device(&features(queues)).unwrap()
}

pub fn try_device(queues: u32) -> Option<Host> {
    let have = mk::Device::supported_features().ok()?;
    (have.queues >= queues).then(|| device(queues))
}

fn features(queues: u32) -> mk::DeviceInfo {
    mk::DeviceInfo {
        queues,
        ..mk::Device::supported_features().unwrap()
    }
}
