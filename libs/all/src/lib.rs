use jni::objects::{JClass, JString};
use jni::{JNIEnv, NativeMethod};

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dov_mk_Natives_bind(mut env: JNIEnv, _: JClass, backend: JString) {
    let backend: String = env.get_string(&backend).unwrap().into();
    let natives = match backend.as_str() {
        "vk" => vk::java::natives(),
        #[cfg(target_os = "macos")]
        "mtl" => mtl::java::natives(),
        "gl" => gl::java::natives(),
        _ => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("no backend {backend}"),
            );
            return;
        }
    };
    for (class, name, sig, fn_ptr) in natives {
        let method = NativeMethod {
            name: name.into(),
            sig: sig.into(),
            fn_ptr,
        };
        if env.register_native_methods(class, &[method]).is_err() {
            return;
        }
    }
}
