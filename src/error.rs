#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported(String),
    OutOfMemory(String),
    DeviceLost(String),
    Shader(String),
    Invalid(String),
    Driver(String),
    OutOfDate(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Unsupported(m) => write!(f, "unsupported: {m}"),
            Error::OutOfMemory(m) => write!(f, "out of memory: {m}"),
            Error::DeviceLost(m) => write!(f, "device lost: {m}"),
            Error::Shader(m) => write!(f, "shader: {m}"),
            Error::Invalid(m) => write!(f, "invalid: {m}"),
            Error::Driver(m) => write!(f, "driver: {m}"),
            Error::OutOfDate(m) => write!(f, "out of date: {m}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(feature = "vk")]
impl From<ash::vk::Result> for Error {
    fn from(r: ash::vk::Result) -> Self {
        use ash::vk::Result as R;
        match r {
            R::ERROR_OUT_OF_HOST_MEMORY
            | R::ERROR_OUT_OF_DEVICE_MEMORY
            | R::ERROR_OUT_OF_POOL_MEMORY
            | R::ERROR_FRAGMENTED_POOL
            | R::ERROR_FRAGMENTATION => Error::OutOfMemory(r.to_string()),
            R::ERROR_DEVICE_LOST => Error::DeviceLost(r.to_string()),
            R::ERROR_OUT_OF_DATE_KHR => Error::OutOfDate(r.to_string()),
            R::ERROR_FEATURE_NOT_PRESENT
            | R::ERROR_EXTENSION_NOT_PRESENT
            | R::ERROR_LAYER_NOT_PRESENT
            | R::ERROR_INCOMPATIBLE_DRIVER
            | R::ERROR_FORMAT_NOT_SUPPORTED => Error::Unsupported(r.to_string()),
            r => Error::Driver(r.to_string()),
        }
    }
}

#[cfg(feature = "mtl")]
impl From<objc2::rc::Retained<objc2_foundation::NSError>> for Error {
    fn from(e: objc2::rc::Retained<objc2_foundation::NSError>) -> Self {
        Error::Driver(e.localizedDescription().to_string())
    }
}
