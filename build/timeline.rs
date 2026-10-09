
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nValue(mut env: JNIEnv, _: JClass, timeline: jlong) -> jlong {
    crate::java::call(&mut env, |_| Ok(crate::java::timeline(timeline).value()? as jlong))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nWait(mut env: JNIEnv, _: JClass, timeline: jlong, value: jlong, ms: jlong) -> jboolean {
    crate::java::call(&mut env, |_| Ok(crate::java::timeline(timeline).wait(value as u64, ms as u64)? as jboolean))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nSignal(mut env: JNIEnv, _: JClass, timeline: jlong, value: jlong) {
    crate::java::call(&mut env, |_| crate::java::timeline(timeline).signal(value as u64))
}

#[cfg(feature = "vk")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nRaw(mut env: JNIEnv, _: JClass, timeline: jlong) -> jlong {
    crate::java::call(&mut env, |_| Ok(ash::vk::Handle::as_raw(crate::java::timeline(timeline).raw()) as jlong))
}

#[cfg(feature = "mtl")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nRaw(mut env: JNIEnv, _: JClass, timeline: jlong) -> jlong {
    crate::java::call(&mut env, |_| Ok(crate::java::pointer(crate::java::timeline(timeline).raw())))
}

#[cfg(feature = "gl")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Timeline_nRaw(mut env: JNIEnv, _: JClass, _timeline: jlong) -> jlong {
    crate::java::call(&mut env, |_| Err(crate::Error::Unsupported("GL timelines have no native object".into())))
}
