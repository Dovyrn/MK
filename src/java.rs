use crate::Device;
use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::{jboolean, jint, jlong};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn call<'local, T: Default>(
    env: &mut JNIEnv<'local>,
    f: impl FnOnce(&mut JNIEnv<'local>) -> Result<T, String>,
) -> T {
    let message = match catch_unwind(AssertUnwindSafe(|| f(env))) {
        Ok(Ok(v)) => return v,
        Ok(Err(e)) => e,
        Err(p) => p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "panic".into()),
    };
    let _ = env.throw_new("java/lang/RuntimeException", message);
    T::default()
}

pub fn device<'a>(handle: jlong) -> &'a mut Device {
    assert!(handle != 0, "device used after free");
    unsafe { &mut *(handle as *mut Device) }
}

pub fn bytes<'a>(address: jlong, length: i64) -> &'a [u8] {
    if address == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(address as *const u8, length as usize) }
    }
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn bytes_mut<'a>(address: jlong, length: i64) -> &'a mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(address as *mut u8, length as usize) }
}

pub fn str<'a>(address: jlong, length: i64) -> &'a str {
    std::str::from_utf8(bytes(address, length)).unwrap()
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn u32s<'a>(address: jlong, count: i64) -> &'a [u32] {
    unsafe { std::slice::from_raw_parts(address as *const u32, count as usize) }
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn f32x12<'a>(address: jlong) -> &'a [f32; 12] {
    unsafe { &*(address as *const [f32; 12]) }
}

pub fn waits(address: jlong, count: i64) -> Vec<(u32, u64)> {
    let mut b = bytes(address, count * 16);
    (0..count)
        .map(|_| {
            let event = crate::wire::u32(&mut b);
            crate::wire::u32(&mut b);
            (event, crate::wire::u64(&mut b))
        })
        .collect()
}

#[cfg(feature = "vk")]
pub fn pairs(address: jlong, count: i64) -> Vec<(u32, u32)> {
    u32s(address, count * 2)
        .chunks(2)
        .map(|p| (p[0], p[1]))
        .collect()
}

#[cfg(feature = "mtl")]
pub fn f32s<'a>(address: jlong, count: i64) -> &'a [f32] {
    unsafe { std::slice::from_raw_parts(address as *const f32, count as usize) }
}

#[cfg(feature = "mtl")]
pub fn strs<'a>(b: &mut &'a [u8]) -> Vec<&'a str> {
    (0..crate::wire::u32(b))
        .map(|_| crate::wire::str(b))
        .collect()
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn list<T>(b: &mut &[u8], f: impl Fn(&mut &[u8]) -> T) -> Vec<T> {
    (0..crate::wire::u32(b)).map(|_| f(b)).collect()
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn u32s_out(out: jlong, v: &[u32]) {
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), out as *mut u32, v.len()) };
}

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn pair(out: jlong, v: (u64, u64)) {
    unsafe { *(out as *mut [u64; 2]) = [v.0, v.1] };
}

pub fn write(out: jlong, f: impl FnOnce(&mut Vec<u8>)) {
    let mut v = Vec::new();
    f(&mut v);
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), out as *mut u8, v.len()) };
}

#[cfg(feature = "mtl")]
pub fn object<T: objc2::Message>(address: jlong) -> objc2::rc::Retained<T> {
    unsafe { objc2::rc::Retained::retain(address as *mut T).unwrap() }
}

#[cfg(feature = "mtl")]
pub fn pointer<T: ?Sized>(r: &T) -> jlong {
    std::ptr::from_ref(r).cast::<u8>() as jlong
}

include!(concat!(env!("OUT_DIR"), "/java.rs"));
