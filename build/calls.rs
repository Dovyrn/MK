use crate::{camel, pascal};
use std::fmt::Write;

pub struct Param {
    pub name: String,
    pub kind: String,
}

pub struct Call {
    pub name: String,
    pub target: String,
    pub backend: Option<String>,
    pub params: Vec<Param>,
    pub result: String,
    pub fails: bool,
}

pub fn parse(text: &str) -> Vec<Call> {
    let table: toml::Table = text.parse().unwrap();
    table["call"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| Call {
            name: c["name"].as_str().unwrap().to_string(),
            target: c
                .get("target")
                .unwrap_or(&c["name"])
                .as_str()
                .unwrap()
                .to_string(),
            backend: c.get("backend").map(|b| b.as_str().unwrap().to_string()),
            params: c
                .get("params")
                .map(|p| {
                    p.as_array()
                        .unwrap()
                        .iter()
                        .map(|p| {
                            let p = p.as_array().unwrap();
                            Param {
                                name: p[0].as_str().unwrap().to_string(),
                                kind: p[1].as_str().unwrap().to_string(),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            result: c
                .get("result")
                .map_or("", |r| r.as_str().unwrap())
                .to_string(),
            fails: c.get("fails").is_some_and(|f| f.as_bool().unwrap()),
        })
        .collect()
}

fn class(call: &Call) -> &str {
    match call.backend.as_deref() {
        Some("vk") => "VkDevice",
        Some("mtl") => "MtlDevice",
        _ => "Device",
    }
}

fn info(kind: &str) -> bool {
    kind.chars().next().is_some_and(char::is_uppercase)
}

fn method(call: &Call) -> String {
    if call.name == "new" {
        "create".into()
    } else {
        camel(&call.name)
    }
}

fn native(call: &Call) -> String {
    format!("n{}", pascal(&call.name))
}

fn statics(call: &Call) -> bool {
    call.result == "device"
}

fn natives(p: &Param) -> Vec<(String, String, &'static str, &'static str)> {
    let n = camel(&p.name);
    let r = p.name.clone();
    let one = |t: &'static str, j: &'static str| vec![(n.clone(), r.clone(), t, j)];
    let two = |t: &'static str, j: &'static str| {
        vec![
            (n.clone(), r.clone(), "long", "jlong"),
            (format!("{n}Length"), format!("{r}_length"), t, j),
        ]
    };
    match p.kind.as_str() {
        "u32" | "usize" | "i32" => one("int", "jint"),
        "u64" | "handle" | "object" | "f32x12" | "pointer" => one("long", "jlong"),
        "f32" => one("float", "jfloat"),
        "bool" => one("boolean", "jboolean"),
        "bytes" | "mutbytes" | "commands" => two("long", "jlong"),
        _ => two("int", "jint"),
    }
}

fn outs(call: &Call) -> bool {
    matches!(
        call.result.as_str(),
        "pair" | "handles" | "pair32" | "u32x2" | "u32x3"
    ) || info(&call.result) && call.result != "device"
}

fn result_types(call: &Call) -> (&'static str, &'static str) {
    match call.result.as_str() {
        "u32" | "i32" => ("int", "jint"),
        "u64" | "pointer" | "handle" | "object" | "device" => ("long", "jlong"),
        "bytes" => ("byte[]", "jni::objects::JByteArray<'local>"),
        "bool" => ("boolean", "jboolean"),
        _ => ("void", "()"),
    }
}

fn rust_arg(p: &Param) -> String {
    let n = p.name.clone();
    let j = "crate::java";
    match p.kind.as_str() {
        "u32" => format!("{n} as u32"),
        "i32" => n,
        "pointer" => format!("{n} as usize as *mut u8"),
        "f32s" => format!("{j}::f32s({n}, {n}_length as i64)"),
        "[str]" => format!("&{j}::strs(&mut {j}::bytes({n}, {n}_length as i64))"),
        "usize" => format!("{n} as usize"),
        "u64" => format!("{n} as u64"),
        "f32" => n,
        "bool" => format!("{n} != 0"),
        "str" => format!("{j}::str({n}, {n}_length as i64)"),
        "bytes" | "commands" => format!("{j}::bytes({n}, {n}_length)"),
        "mutbytes" => format!("{j}::bytes_mut({n}, {n}_length)"),
        "u32s" => format!("{j}::u32s({n}, {n}_length as i64)"),
        "f32x12" => format!("{j}::f32x12({n})"),
        "waits" => format!("&{j}::waits({n}, {n}_length as i64)"),
        "pairs" => format!("&{j}::pairs({n}, {n}_length as i64)"),
        "handle" => format!("ash::vk::Handle::from_raw({n} as u64)"),
        "object" => format!("{j}::object({n})"),
        k if k.starts_with('[') => format!(
            "&{j}::list(&mut {j}::bytes({n}, {n}_length as i64), crate::{}::read)",
            &k[1..k.len() - 1]
        ),
        k => format!("&crate::{k}::read(&mut {j}::bytes({n}, {n}_length as i64))"),
    }
}

pub fn rust(calls: &[Call], borrowing: &[String]) -> String {
    let mut s = String::from(
        "#[unsafe(no_mangle)]\npub extern \"system\" fn Java_dev_dov_mk_Device_nFree(_: JNIEnv, _: JClass, device: jlong) {\n",
    );
    s += "    drop(unsafe { Box::from_raw(device as *mut crate::Device) });\n}\n";
    for call in calls {
        let mut params = vec![
            "mut env: JNIEnv<'local>".to_string(),
            "_: JClass".to_string(),
        ];
        if !statics(call) {
            params.push("device: jlong".into());
        }
        for p in &call.params {
            for (_, n, _, t) in natives(p) {
                params.push(format!("{n}: {t}"));
            }
        }
        if outs(call) {
            params.push("out: jlong".into());
        }
        let (_, ret) = result_types(call);
        let ret = if ret == "()" {
            String::new()
        } else {
            format!(" -> {ret}")
        };
        s += "\n";
        if let Some(b) = &call.backend {
            writeln!(s, "#[cfg(feature = \"{b}\")]").unwrap();
        }
        writeln!(
            s,
            "#[unsafe(no_mangle)]\npub extern \"system\" fn Java_dev_dov_mk_{}_{}<'local>({}){ret} {{",
            class(call),
            native(call),
            params.join(", ")
        )
        .unwrap();
        let args: Vec<_> = call.params.iter().map(rust_arg).collect();
        let target = if statics(call) {
            format!("crate::Device::{}", call.target)
        } else {
            format!("crate::java::device(device).{}", call.target)
        };
        let borrowed = call.params.iter().find(|p| borrowing.contains(&p.kind));
        let invoke = match borrowed {
            Some(p) => {
                let n = p.name.clone();
                let inner: Vec<_> = call
                    .params
                    .iter()
                    .zip(&args)
                    .map(|(q, a)| {
                        if q.name == p.name {
                            "info".to_string()
                        } else {
                            a.clone()
                        }
                    })
                    .collect();
                format!(
                    "crate::{}::decode(&mut crate::java::bytes({n}, {n}_length as i64), |info| {target}({}))",
                    p.kind,
                    inner.join(", ")
                )
            }
            None => format!("{target}({})", args.join(", ")),
        };
        let invoke = if call.fails {
            format!("{invoke}.map_err(|e| e.to_string())?")
        } else {
            invoke
        };
        let body = match call.result.as_str() {
            "" => format!("{invoke};\n        Ok(())"),
            "u32" | "i32" => format!("Ok({invoke} as jint)"),
            "pair32" => format!(
                "let r = {invoke};\n        crate::java::pair(out, (r.0 as u64, r.1 as u64));\n        Ok(())"
            ),
            "u32x2" | "u32x3" => format!("crate::java::u32s_out(out, &{invoke});\n        Ok(())"),
            "bytes" => format!(
                "let r = {invoke};\n        env.byte_array_from_slice(&r).map_err(|e| e.to_string())"
            ),
            "u64" => format!("Ok({invoke} as jlong)"),
            "bool" => format!("Ok({invoke} as jboolean)"),
            "pointer" => format!("Ok({invoke} as usize as jlong)"),
            "handle" => format!("Ok(ash::vk::Handle::as_raw({invoke}) as jlong)"),
            "object" => format!("Ok(crate::java::pointer({invoke}))"),
            "device" => format!("Ok(Box::into_raw(Box::new({invoke})) as jlong)"),
            "pair" => format!("crate::java::pair(out, {invoke});\n        Ok(())"),
            "handles" => format!(
                "let r = {invoke};\n        crate::java::pair(out, (ash::vk::Handle::as_raw(r.0), ash::vk::Handle::as_raw(r.1)));\n        Ok(())"
            ),
            _ => format!(
                "let r = {invoke};\n        crate::java::write(out, |o| r.write(o));\n        Ok(())"
            ),
        };
        writeln!(
            s,
            "    crate::java::call(&mut env, |env| {{\n        let _ = &env;\n        {body}\n    }})\n}}"
        )
        .unwrap();
    }
    s
}

pub fn java(calls: &[Call], backend: Option<&str>) -> String {
    let name = match backend {
        Some("vk") => "VkDevice",
        Some("mtl") => "MtlDevice",
        _ => "Device",
    };
    let mut s =
        String::from("package dev.dov.mk;\n\nimport static org.lwjgl.system.MemoryUtil.*;\n\n");
    s += "import java.nio.ByteBuffer;\nimport java.nio.charset.StandardCharsets;\nimport org.lwjgl.system.MemoryStack;\n\n";
    if backend.is_none() {
        s += "public class Device {\n    public final long handle;\n\n";
        s += "    public Device(long handle) {\n        this.handle = handle;\n    }\n\n";
        s += "    public void free() {\n        nFree(handle);\n    }\n\n    private static native void nFree(long device);\n";
    } else {
        s += "import lombok.experimental.UtilityClass;\n\n";
        writeln!(s, "@UtilityClass\npublic class {name} {{").unwrap();
    }
    for call in calls.iter().filter(|c| c.backend.as_deref() == backend) {
        let mut params = Vec::new();
        if !statics(call) && backend.is_some() {
            params.push("Device device".to_string());
        }
        for p in &call.params {
            let t = match p.kind.as_str() {
                "u32" | "usize" | "i32" => "int",
                "u64" | "handle" | "object" | "pointer" => "long",
                "f32s" => "float[]",
                "[str]" => "String[]",
                "f32" => "float",
                "bool" => "boolean",
                "str" => "String",
                "bytes" | "mutbytes" => "ByteBuffer",
                "commands" => "Commands",
                "u32s" | "pairs" => "int[]",
                "f32x12" => "float[]",
                "waits" => "long[]",
                k if k.starts_with('[') => &k[1..k.len() - 1],
                k => k,
            };
            let t = if p.kind.starts_with('[') && p.kind != "[str]" {
                format!("{t}[]")
            } else {
                t.to_string()
            };
            params.push(format!("{t} {}", camel(&p.name)));
        }
        let (java_ret, _) = result_types(call);
        let ret = match call.result.as_str() {
            "pair" | "handles" | "pair32" => "long[]".to_string(),
            "u32x2" | "u32x3" => "int[]".into(),
            "device" => "Device".into(),
            r if info(r) => r.into(),
            _ => java_ret.into(),
        };
        let modifier = if statics(call) || backend.is_some() {
            "static "
        } else {
            ""
        };
        writeln!(
            s,
            "\n    public {modifier}{ret} {}({}) {{",
            method(call),
            params.join(", ")
        )
        .unwrap();
        s += "        try (var stack = MemoryStack.stackPush()) {\n";
        let mut args = Vec::new();
        if !statics(call) {
            args.push(
                if backend.is_some() {
                    "device.handle"
                } else {
                    "handle"
                }
                .to_string(),
            );
        }
        for p in &call.params {
            let n = camel(&p.name);
            match p.kind.as_str() {
                "u32" | "usize" | "i32" | "u64" | "handle" | "object" | "pointer" | "f32"
                | "bool" => args.push(n),
                "f32s" => {
                    args.push(format!("memAddress(stack.floats({n}))"));
                    args.push(format!("{n}.length"));
                }
                "[str]" => {
                    writeln!(
                        s,
                        "            var {n}Size = 4;\n            for (var x : {n}) {{\n                {n}Size += 4 + x.getBytes(StandardCharsets.UTF_8).length;\n            }}\n            var {n}Address = stack.nmalloc(8, {n}Size);\n            memPutInt({n}Address, {n}.length);\n            var {n}At = {n}Address + 4;\n            for (var x : {n}) {{\n                var b = x.getBytes(StandardCharsets.UTF_8);\n                memPutInt({n}At, b.length);\n                memByteBuffer({n}At + 4, b.length).put(b);\n                {n}At += 4 + b.length;\n            }}"
                    )
                    .unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}Size"));
                }
                k if k.starts_with('[') => {
                    writeln!(
                        s,
                        "            var {n}Size = 4;\n            for (var x : {n}) {{\n                {n}Size += x.size();\n            }}\n            var {n}Address = stack.nmalloc(8, {n}Size);\n            memPutInt({n}Address, {n}.length);\n            var {n}At = {n}Address + 4;\n            for (var x : {n}) {{\n                {n}At = x.write({n}At);\n            }}"
                    )
                    .unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}Size"));
                }
                "str" => {
                    writeln!(s, "            var {n}Bytes = stack.UTF8({n}, false);").unwrap();
                    args.push(format!("memAddress({n}Bytes)"));
                    args.push(format!("{n}Bytes.remaining()"));
                }
                "bytes" | "mutbytes" => {
                    args.push(format!("memAddress({n})"));
                    args.push(format!("{n}.remaining()"));
                }
                "commands" => {
                    args.push(format!("{n}.getAddress()"));
                    args.push(format!("{n}.getSize()"));
                }
                "u32s" | "pairs" => {
                    args.push(format!("memAddress(stack.ints({n}))"));
                    args.push(if p.kind == "pairs" {
                        format!("{n}.length / 2")
                    } else {
                        format!("{n}.length")
                    });
                }
                "f32x12" => args.push(format!("memAddress(stack.floats({n}))")),
                "waits" => {
                    writeln!(
                        s,
                        "            var {n}Address = stack.nmalloc(8, {n}.length * 8);\n            for (var i = 0; i < {n}.length / 2; i++) {{\n                memPutInt({n}Address + i * 16, (int) {n}[i * 2]);\n                memPutLong({n}Address + i * 16 + 8, {n}[i * 2 + 1]);\n            }}"
                    )
                    .unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}.length / 2"));
                }
                _ => {
                    writeln!(s, "            var {n}Address = stack.nmalloc(8, {n}.size());\n            {n}.write({n}Address);").unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}.size()"));
                }
            }
        }
        if outs(call) {
            s += "            var out = stack.nmalloc(8, 256);\n";
            args.push("out".into());
        }
        let invoke = format!("{}({})", native(call), args.join(", "));
        let line = match call.result.as_str() {
            "" => format!("{invoke};"),
            "pair" | "handles" | "pair32" => format!(
                "{invoke};\n            return new long[] {{memGetLong(out), memGetLong(out + 8)}};"
            ),
            "u32x2" => format!(
                "{invoke};\n            return new int[] {{memGetInt(out), memGetInt(out + 4)}};"
            ),
            "u32x3" => format!(
                "{invoke};\n            return new int[] {{memGetInt(out), memGetInt(out + 4), memGetInt(out + 8)}};"
            ),
            "device" => format!("return new Device({invoke});"),
            r if info(r) => format!("{invoke};\n            return {r}.read(out);"),
            _ => format!("return {invoke};"),
        };
        writeln!(s, "            {line}\n        }}\n    }}").unwrap();
        let mut natives_params = Vec::new();
        if !statics(call) {
            natives_params.push("long device".to_string());
        }
        for p in &call.params {
            for (n, _, t, _) in natives(p) {
                natives_params.push(format!("{t} {n}"));
            }
        }
        if outs(call) {
            natives_params.push("long out".into());
        }
        writeln!(
            s,
            "\n    private static native {java_ret} {}({});",
            native(call),
            natives_params.join(", ")
        )
        .unwrap();
    }
    s += "}\n";
    s
}
