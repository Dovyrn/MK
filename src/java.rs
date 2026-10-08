use crate::{CommandBuffer, ComputePass, Device, RenderPass, Timeline};
use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::{jboolean, jfloat, jint, jlong};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn call<'local, T: Default>(
    env: &mut JNIEnv<'local>,
    f: impl FnOnce(&mut JNIEnv<'local>) -> Result<T, String>,
) -> T {
    let (class, message) = match catch_unwind(AssertUnwindSafe(|| f(env))) {
        Ok(Ok(v)) => return v,
        Ok(Err(e)) => ("java/lang/RuntimeException", e),
        Err(p) => (
            "java/lang/AssertionError",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "panic".into()),
        ),
    };
    if !env.exception_check().unwrap_or(false) {
        let _ = env.throw_new(class, message);
    }
    T::default()
}

pub fn device<'a>(handle: jlong) -> &'a mut Device {
    assert_ne!(handle, 0, "device used after close");
    unsafe { &mut *(handle as *mut Device) }
}

pub fn shared<'a>(handle: jlong) -> &'a Device {
    assert_ne!(handle, 0, "device used after close");
    unsafe { &*(handle as *const Device) }
}

pub fn new_command_buffer(commands: CommandBuffer<'_>) -> jlong {
    let commands: CommandBuffer<'static> = unsafe { std::mem::transmute(commands) };
    Box::into_raw(Box::new(commands)) as jlong
}

pub fn new_compute_pass(pass: ComputePass<'_, '_>) -> jlong {
    let pass: ComputePass<'static, 'static> = unsafe { std::mem::transmute(pass) };
    Box::into_raw(Box::new(pass)) as jlong
}

pub fn new_render_pass(pass: RenderPass<'_, '_>) -> jlong {
    let pass: RenderPass<'static, 'static> = unsafe { std::mem::transmute(pass) };
    Box::into_raw(Box::new(pass)) as jlong
}

pub fn command_buffer<'a>(handle: jlong) -> &'a mut CommandBuffer<'static> {
    unsafe { &mut *(handle as *mut CommandBuffer<'static>) }
}

pub fn compute_pass<'a>(handle: jlong) -> &'a mut ComputePass<'static, 'static> {
    unsafe { &mut *(handle as *mut ComputePass<'static, 'static>) }
}

pub fn render_pass<'a>(handle: jlong) -> &'a mut RenderPass<'static, 'static> {
    unsafe { &mut *(handle as *mut RenderPass<'static, 'static>) }
}

pub fn take_commands(handle: jlong) -> CommandBuffer<'static> {
    *unsafe { Box::from_raw(handle as *mut CommandBuffer<'static>) }
}

#[cfg(feature = "vk")]
fn raw_commands(raw: ash::vk::CommandBuffer) -> jlong {
    ash::vk::Handle::as_raw(raw) as jlong
}

#[cfg(feature = "mtl")]
fn raw_commands<T: ?Sized>(raw: &T) -> jlong {
    pointer(raw)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_CommandBuffer_nBeginComputePass(
    mut env: JNIEnv,
    _: JClass,
    commands: jlong,
) -> jlong {
    call(&mut env, |_| {
        Ok(new_compute_pass(
            command_buffer(commands).begin_compute_pass()?,
        ))
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_CommandBuffer_nBeginRenderPass(
    mut env: JNIEnv,
    _: JClass,
    commands: jlong,
    info: jlong,
    length: jint,
) -> jlong {
    call(&mut env, |_| {
        let commands = command_buffer(commands);
        let pass = crate::RenderPassInfo::decode(&mut bytes(info, length as i64), |info| {
            commands.begin_render_pass(info)
        })?;
        Ok(new_render_pass(pass))
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_CommandBuffer_nFinish(
    mut env: JNIEnv,
    _: JClass,
    commands: jlong,
) {
    call(&mut env, |_| take_commands(commands).finish())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_CommandBuffer_nDrop(
    mut env: JNIEnv,
    _: JClass,
    commands: jlong,
) {
    call(&mut env, |_| {
        drop(take_commands(commands));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_CommandBuffer_nRaw(
    mut env: JNIEnv,
    _: JClass,
    commands: jlong,
) -> jlong {
    call(&mut env, |_| {
        let _ = commands;
        #[cfg(any(feature = "vk", feature = "mtl"))]
        return Ok(raw_commands(command_buffer(commands).raw()));
        #[cfg(feature = "gl")]
        Err("GL has no command buffers".to_string())
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_ComputePass_nEnd(mut env: JNIEnv, _: JClass, pass: jlong) {
    call(&mut env, |_| {
        drop(unsafe { Box::from_raw(pass as *mut ComputePass<'static, 'static>) });
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_RenderPass_nEnd(mut env: JNIEnv, _: JClass, pass: jlong) {
    call(&mut env, |_| {
        drop(unsafe { Box::from_raw(pass as *mut RenderPass<'static, 'static>) });
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_ComputePass_nRaw(
    mut env: JNIEnv,
    _: JClass,
    pass: jlong,
) -> jlong {
    call(&mut env, |_| {
        let _ = pass;
        #[cfg(any(feature = "vk", feature = "mtl"))]
        return Ok(raw_commands(compute_pass(pass).raw()));
        #[cfg(feature = "gl")]
        Err("GL has no encoders".to_string())
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_RenderPass_nRaw(
    mut env: JNIEnv,
    _: JClass,
    pass: jlong,
) -> jlong {
    call(&mut env, |_| {
        let _ = pass;
        #[cfg(any(feature = "vk", feature = "mtl"))]
        return Ok(raw_commands(render_pass(pass).raw()));
        #[cfg(feature = "gl")]
        Err("GL has no encoders".to_string())
    })
}

pub fn bytes<'a>(address: jlong, length: i64) -> &'a [u8] {
    assert!(length >= 0, "size {length} is negative");
    if length == 0 {
        return &[];
    }
    assert_ne!(address, 0, "a null pointer with {length} bytes");
    unsafe { std::slice::from_raw_parts(address as *const u8, length as usize) }
}

pub fn bytes_mut<'a>(address: jlong, length: i64) -> &'a mut [u8] {
    assert!(
        length >= 0 && address != 0,
        "a write target {address:#x} with size {length}"
    );
    unsafe { std::slice::from_raw_parts_mut(address as *mut u8, length as usize) }
}

pub fn str<'a>(address: jlong, length: i64) -> &'a str {
    std::str::from_utf8(bytes(address, length)).unwrap()
}

pub fn waits<'a>(address: jlong, count: i64) -> Vec<(&'a Timeline, u64)> {
    let mut b = bytes(address, count * 16);
    (0..count)
        .map(|_| {
            let t = timeline(crate::wire::u64(&mut b) as jlong);
            (t, crate::wire::u64(&mut b))
        })
        .collect()
}

pub fn timeline<'a>(handle: jlong) -> &'a Timeline {
    assert_ne!(handle, 0, "timeline used after destroy");
    unsafe { &*(handle as *const Timeline) }
}

pub fn take(handle: jlong) -> Timeline {
    assert_ne!(handle, 0, "timeline used after destroy");
    *unsafe { Box::from_raw(handle as *mut Timeline) }
}

#[cfg(feature = "vk")]
pub fn pairs(address: jlong, count: i64) -> Vec<(u32, u32)> {
    assert!(count >= 0, "count {count} is negative");
    if count == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(address as *const u32, count as usize * 2) }
        .chunks(2)
        .map(|p| (p[0], p[1]))
        .collect()
}

#[cfg(feature = "vk")]
pub fn pair(out: jlong, v: (u64, u64)) {
    unsafe { *(out as *mut [u64; 2]) = [v.0, v.1] };
}

pub fn u32s_out(out: jlong, v: &[u32]) {
    unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), out as *mut u32, v.len()) };
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
include!(concat!(env!("OUT_DIR"), "/commands_jni.rs"));
