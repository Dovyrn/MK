use crate::field::Field;
use crate::infos::{java_type, list_put, list_size, reader};
use crate::{camel, class, enumerated, flags, handle, list, member, pascal, word};
use serde::Deserialize;
use std::fmt::Write;

#[derive(Deserialize)]
pub struct Call {
    name: String,
    backend: Option<String>,
    #[serde(default)]
    params: Vec<Field>,
    #[serde(default)]
    result: String,
    #[serde(default)]
    fails: bool,
    #[serde(default)]
    shared: bool,
    #[serde(default, rename = "static")]
    fixed: bool,
}

const RECORDINGS: [&str; 3] = ["CommandBuffer", "RenderPass", "ComputePass"];

fn info(kind: &str) -> bool {
    kind.chars().next().is_some_and(char::is_uppercase)
        && !handle(kind)
        && !word(kind)
        && !RECORDINGS.contains(&kind)
}

fn method(call: &Call) -> String {
    camel(&call.name)
}

fn native(call: &Call) -> String {
    format!("n{}", pascal(&call.name))
}

fn statics(call: &Call) -> bool {
    call.result == "device" || call.fixed
}

fn natives(p: &Field) -> Vec<(String, String, &'static str, &'static str)> {
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
        k if word(k) => one("int", "jint"),
        k if handle(k) => one("long", "jlong"),
        "u64" | "handle" | "object" | "objref" | "f32x12" | "pointer" | "timeline" | "taken" => {
            one("long", "jlong")
        }
        "f32" => one("float", "jfloat"),
        "bool" => one("boolean", "jboolean"),
        "bytes" | "mutbytes" => two("long", "jlong"),
        _ => two("int", "jint"),
    }
}

fn outs(call: &Call) -> bool {
    matches!(call.result.as_str(), "pair" | "handles" | "u32x3" | "u32x8")
        || info(&call.result) && call.result != "device"
}

fn result_types(call: &Call) -> (&'static str, &'static str) {
    match call.result.as_str() {
        "u32" | "i32" => ("int", "jint"),
        k if flags(k) => ("int", "jint"),
        k if handle(k) || RECORDINGS.contains(&k) => ("long", "jlong"),
        "u64" | "pointer" | "handle" | "object" | "device" | "timeline" => ("long", "jlong"),
        "str" => ("String", "jni::objects::JString<'local>"),
        "bool" => ("boolean", "jboolean"),
        _ => ("void", "()"),
    }
}

fn rust_arg(p: &Field) -> String {
    let n = p.name.clone();
    let j = "crate::java";
    match p.kind.as_str() {
        k if handle(k) => format!("crate::{k}::from_raw({n} as u64)"),
        k if word(k) => format!("crate::{k}::of({n} as u32)"),
        "u32" => format!("{n} as u32"),
        "i32" => n,
        "pointer" => format!("{n} as usize as *mut u8"),
        "usize" => format!("{n} as usize"),
        "u64" => format!("{n} as u64"),
        "f32" => n,
        "bool" => format!("{n} != 0"),
        "str" => format!("{j}::str({n}, {n}_length as i64)"),
        "bytes" => format!("{j}::bytes({n}, {n}_length)"),
        "mutbytes" => format!("{j}::bytes_mut({n}, {n}_length)"),
        "commands" => format!("{j}::commands({n}, {n}_length as i64)"),
        "f32x12" => format!("{j}::f32x12({n})"),
        "waits" => format!("&{j}::waits({n}, {n}_length as i64)"),
        "pairs" => format!("&{j}::pairs({n}, {n}_length as i64)"),
        "handle" => format!("ash::vk::Handle::from_raw({n} as u64)"),
        "object" => format!("{j}::object({n})"),
        "objref" => format!("&{j}::object({n})"),
        "timeline" => format!("{j}::timeline({n})"),
        "taken" => format!("{j}::take({n})"),
        k => match list(k) {
            Some(item) => format!(
                "&crate::wire::list(&mut {j}::bytes({n}, {n}_length as i64), {})",
                reader(item)
            ),
            None => format!("&{}(&mut {j}::bytes({n}, {n}_length as i64))", reader(k)),
        },
    }
}

pub fn rust(calls: &[Call], borrowing: &[String]) -> String {
    let mut s = String::from(
        "#[unsafe(no_mangle)]\npub extern \"system\" fn Java_dev_dov_mk_Device_nFree(mut env: JNIEnv, _: JClass, device: jlong) {\n",
    );
    s += "    crate::java::call(&mut env, |_| {\n        drop(unsafe { Box::from_raw(device as *mut crate::Device) });\n        Ok(())\n    })\n}\n";
    s += include_str!("timeline.rs");
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
        let args: Vec<_> = call.params.iter().map(rust_arg).collect();
        let target = if statics(call) {
            format!("crate::Device::{}", call.name)
        } else if call.shared {
            format!("crate::java::shared(device).{}", call.name)
        } else {
            format!("crate::java::device(device).{}", call.name)
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
            format!("{invoke}?")
        } else {
            invoke
        };
        let body = match call.result.as_str() {
            "" => format!("{invoke};\n        Ok(())"),
            "u32" | "i32" => format!("Ok({invoke} as jint)"),
            k if flags(k) => format!("Ok({invoke}.0 as jint)"),
            k if handle(k) => format!("Ok({invoke}.to_raw() as jlong)"),
            "CommandBuffer" => format!("Ok(crate::java::new_command_buffer({invoke}))"),
            "RenderPass" => format!("Ok(crate::java::new_render_pass({invoke}))"),
            "ComputePass" => format!("Ok(crate::java::new_compute_pass({invoke}))"),
            "u32x3" | "u32x8" => {
                format!("crate::java::u32s_out(out, &{invoke});\n        Ok(())")
            }
            "str" => {
                format!(
                    "let r = {invoke};\n        env.new_string(r).map_err(|e| crate::Error::Driver(e.to_string()))"
                )
            }
            "u64" => format!("Ok({invoke} as jlong)"),
            "bool" => format!("Ok({invoke} as jboolean)"),
            "pointer" => format!(
                "let r: &mut [u8] = unsafe {{ {invoke} }};\n        Ok(r.as_mut_ptr() as usize as jlong)"
            ),
            "handle" => format!("Ok(ash::vk::Handle::as_raw({invoke}) as jlong)"),
            "object" => format!("Ok(crate::java::pointer({invoke}))"),
            "device" | "timeline" => format!("Ok(Box::into_raw(Box::new({invoke})) as jlong)"),
            "pair" => format!("crate::java::pair(out, {invoke});\n        Ok(())"),
            "handles" => format!(
                "let r = {invoke};\n        crate::java::pair(out, (ash::vk::Handle::as_raw(r.0), ash::vk::Handle::as_raw(r.1)));\n        Ok(())"
            ),
            _ => format!(
                "let r = {invoke};\n        crate::java::write(out, |o| r.write(o));\n        Ok(())"
            ),
        };
        let backends: Vec<Option<&str>> = match call.backend.as_deref() {
            Some(b) => b.split(' ').map(Some).collect(),
            None => vec![None],
        };
        for b in backends {
            s += "\n";
            if let Some(b) = b {
                writeln!(s, "#[cfg(feature = \"{b}\")]").unwrap();
            }
            writeln!(
                s,
                "#[unsafe(no_mangle)]\npub extern \"system\" fn Java_dev_dov_mk_{}_{}<'local>({}){ret} {{",
                class(b, "Device"),
                native(call),
                params.join(", ")
            )
            .unwrap();
            writeln!(
                s,
                "    crate::java::call(&mut env, |env| {{\n        let _ = &env;\n        {body}\n    }})\n}}"
            )
            .unwrap();
        }
    }
    s
}

fn direct(kind: &str) -> bool {
    handle(kind)
        || word(kind)
        || matches!(
            kind,
            "u32"
                | "usize"
                | "i32"
                | "u64"
                | "handle"
                | "object"
                | "objref"
                | "pointer"
                | "f32"
                | "bool"
                | "bytes"
                | "mutbytes"
                | "timeline"
                | "taken"
        )
}

pub fn java(calls: &[Call], backend: Option<&str>) -> String {
    let name = class(backend, "Device");
    let mut s =
        String::from("package dev.dov.mk;\n\nimport static org.lwjgl.system.MemoryUtil.*;\n\n");
    s += "import java.nio.charset.StandardCharsets;\n\n";
    if backend.is_none() {
        s += "import java.util.concurrent.atomic.AtomicInteger;\n";
        s += "import java.util.concurrent.locks.ReentrantReadWriteLock;\n\n";
        s += include_str!("device.java");
    } else {
        writeln!(
            s,
            "public class {name} extends Device {{\n    public {name}(long handle) {{\n        super(handle);\n    }}"
        )
        .unwrap();
    }
    for call in calls
        .iter()
        .filter(|c| member(c.backend.as_deref(), backend))
    {
        let mut params = Vec::new();
        for p in &call.params {
            let t = match p.kind.as_str() {
                k if handle(k) => k.to_string(),
                k if flags(k) => "int".into(),
                "usize" | "i32" => "int".into(),
                "handle" | "object" | "objref" | "pointer" => "long".into(),
                "f32x12" => "float[]".into(),
                "bytes" | "mutbytes" => "long".into(),
                "commands" => "CommandBuffer[]".into(),
                "pairs" => "int[]".into(),
                "waits" => "TimelinePoint[]".into(),
                "timeline" | "taken" => "Timeline".into(),
                k => java_type(k),
            };
            params.push(format!("{t} {}", camel(&p.name)));
            if matches!(p.kind.as_str(), "bytes" | "mutbytes") {
                params.push(format!("long {}Size", camel(&p.name)));
            }
        }
        let (java_ret, _) = result_types(call);
        let ret = match call.result.as_str() {
            k if handle(k) || RECORDINGS.contains(&k) => k.to_string(),
            k if flags(k) => "int".into(),
            "pair" | "handles" => "long[]".to_string(),
            "u32x3" | "u32x8" => "int[]".into(),
            "device" => name.clone(),
            "timeline" => "Timeline".into(),
            r if info(r) => r.into(),
            _ => java_ret.into(),
        };
        let modifier = if statics(call) { "static " } else { "" };
        writeln!(
            s,
            "\n    public {modifier}{ret} {}({}) {{",
            method(call),
            params.join(", ")
        )
        .unwrap();
        let start = s.len();
        if !statics(call) && !call.shared {
            writeln!(s, "        idle(\"{}\");", method(call)).unwrap();
        }
        let arena = outs(call) || !call.params.iter().all(|p| direct(&p.kind));
        let indent = if arena { "            " } else { "        " };
        if arena {
            s += "        try (var arena = new Arena()) {\n";
        }
        let mut args = Vec::new();
        if !statics(call) {
            args.push("handle()".to_string());
        }
        for p in &call.params {
            let n = camel(&p.name);
            match p.kind.as_str() {
                k if handle(k) => args.push(format!("{k}.raw({n})")),
                k if enumerated(k) => args.push(format!("{n}.value")),
                k if flags(k) => args.push(n),
                "u32" | "usize" | "i32" | "u64" | "handle" | "object" | "objref" | "pointer"
                | "f32" | "bool" => args.push(n),
                k if list(k).is_some() => {
                    list_size(&mut s, k, &n, "            ");
                    writeln!(
                        s,
                        "            var {n}Address = arena.alloc({n}Size);\n            var {n}At = {n}Address;"
                    )
                    .unwrap();
                    list_put(&mut s, k, &n, &format!("{n}At"), "            ").unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}Size"));
                }
                "str" => {
                    writeln!(
                        s,
                        "            var {n}Bytes = {n}.getBytes(StandardCharsets.UTF_8);"
                    )
                    .unwrap();
                    args.push(format!("arena.bytes({n}Bytes)"));
                    args.push(format!("{n}Bytes.length"));
                }
                "bytes" | "mutbytes" => {
                    args.push(n.clone());
                    args.push(format!("{n}Size"));
                }
                "commands" => {
                    writeln!(
                        s,
                        "            var {n}Address = arena.alloc({n}.length * 8L);\n            for (var i = 0; i < {n}.length; i++) {{\n                memPutLong({n}Address + i * 8, {n}[i].take());\n            }}"
                    )
                    .unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}.length"));
                }
                "pairs" => {
                    writeln!(
                        s,
                        "            if ({n}.length % 2 != 0) {{\n                throw new IllegalArgumentException(\"{n} holds pairs\");\n            }}"
                    )
                    .unwrap();
                    args.push(format!("arena.ints({n})"));
                    args.push(format!("{n}.length / 2"));
                }
                "f32x12" => {
                    writeln!(
                        s,
                        "            if ({n}.length != 12) {{\n                throw new IllegalArgumentException(\"{n} has 12 values\");\n            }}"
                    )
                    .unwrap();
                    args.push(format!("arena.floats({n})"));
                }
                "waits" => {
                    writeln!(
                        s,
                        "            var {n}Address = arena.alloc({n}.length * 16L);\n            for (var i = 0; i < {n}.length; i++) {{\n                memPutLong({n}Address + i * 16, {n}[i].timeline().handle());\n                memPutLong({n}Address + i * 16 + 8, {n}[i].value());\n            }}"
                    )
                    .unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}.length"));
                }
                "timeline" => args.push(format!("{n}.handle()")),
                "taken" => args.push(format!("{n}.take()")),
                _ => {
                    writeln!(s, "            var {n}Address = arena.alloc({n}.encodedSize());\n            {n}.encode({n}Address);").unwrap();
                    args.push(format!("{n}Address"));
                    args.push(format!("{n}.encodedSize()"));
                }
            }
        }
        if outs(call) {
            let size = match call.result.as_str() {
                "pair" | "handles" => "16".to_string(),
                "u32x3" => "12".into(),
                "u32x8" => "32".into(),
                r => format!("{r}.SIZE"),
            };
            writeln!(s, "            var out = arena.alloc({size});").unwrap();
            args.push("out".into());
        }
        let invoke = format!("{}({})", native(call), args.join(", "));
        let line = match call.result.as_str() {
            "" => format!("{invoke};"),
            k if handle(k) => format!("return {k}.of({invoke});"),
            k if RECORDINGS.contains(&k) => format!("return {k}.adopt(this, {invoke});"),
            "pair" | "handles" => format!(
                "{invoke};\n{indent}return new long[] {{memGetLong(out), memGetLong(out + 8)}};"
            ),
            r if r.starts_with("u32x") => {
                let n: usize = r[4..].parse().unwrap();
                let words: Vec<_> = (0..n)
                    .map(|i| format!("memGetInt(out + {})", i * 4))
                    .collect();
                format!(
                    "{invoke};\n{indent}return new int[] {{{}}};",
                    words.join(", ")
                )
            }
            "device" => format!("return new {name}({invoke});"),
            "timeline" => format!("return new Timeline({invoke});"),
            r if info(r) => format!("{invoke};\n{indent}return {r}.decode(out);"),
            _ => format!("return {invoke};"),
        };
        writeln!(s, "{indent}{line}").unwrap();
        if arena {
            s += "        }\n";
        }
        if !statics(call) {
            let kind = if call.shared { "read" } else { "write" };
            let body: String = s[start..].lines().map(|l| format!("    {l}\n")).collect();
            s.truncate(start);
            writeln!(
                s,
                "        lock.{kind}Lock().lock();\n        try {{\n{body}        }} finally {{\n            lock.{kind}Lock().unlock();\n        }}"
            )
            .unwrap();
        }
        s += "    }\n";
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
