pub type Host = mk::Device;

pub fn device(queues: u32) -> Host {
    mk::Device::create_device(queues).unwrap()
}

pub fn try_device(queues: u32) -> Option<Host> {
    mk::Device::create_device(queues).ok()
}
