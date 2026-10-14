use crate::field::Field;
use crate::infos::Info;
use crate::{cfg, list, pascal};
use serde::Deserialize;
use std::fmt::Write;

#[derive(Deserialize)]
pub struct Op {
    pub name: String,
    pub backend: Option<String>,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub internal: bool,
    #[serde(default)]
    pub fails: bool,
    pub info: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

fn find<'a>(infos: &'a [Info], name: &str) -> &'a [Field] {
    &infos
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no info {name}"))
        .fields
}

pub fn resolve(ops: &mut [Op], infos: &[Info]) {
    for op in ops {
        if let Some(i) = &op.info {
            op.fields = find(infos, i).to_vec();
        }
    }
}

pub fn rust_type(kind: &str) -> String {
    if let Some(inner) = crate::optional(kind) {
        return format!("Option<{}>", rust_type(inner));
    }
    match kind {
        "str" => "&'a str".into(),
        "f32x4" => "[f32; 4]".into(),
        k => match list(k) {
            Some(item) => format!("&'a [crate::{item}]"),
            None if k.chars().next().unwrap().is_uppercase() => format!("crate::{k}"),
            None => k.into(),
        },
    }
}

fn gate(op: &Op, indent: &str) -> String {
    match cfg(op.backend.as_deref()) {
        Some(c) => format!("{indent}#[cfg({c})]\n"),
        None => String::new(),
    }
}

pub fn command(ops: &[Op]) -> String {
    let mut s = String::from("#[derive(Debug, PartialEq)]\npub enum Command<'a> {\n");
    for op in ops {
        s += &gate(op, "    ");
        if op.fields.is_empty() {
            writeln!(s, "    {},", pascal(&op.name)).unwrap();
        } else {
            let fields: Vec<_> = op
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, rust_type(&f.kind)))
                .collect();
            writeln!(s, "    {} {{ {} }},", pascal(&op.name), fields.join(", ")).unwrap();
        }
    }
    s += "}\n\nimpl Command<'_> {\n    pub fn scope(&self) -> &'static str {\n        match self {\n";
    for op in ops {
        s += &gate(op, "            ");
        writeln!(
            s,
            "            Self::{} {{ .. }} => {:?},",
            pascal(&op.name),
            op.scope
        )
        .unwrap();
    }
    s += "        }\n    }\n}\n\n";
    s += "impl std::fmt::Display for Command<'_> {\n";
    s += "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n";
    s += "        match self {\n";
    for op in ops {
        s += &gate(op, "            ");
        let names: Vec<_> = op.fields.iter().map(|f| f.name.clone()).collect();
        let shown: String = op
            .fields
            .iter()
            .map(|f| match f.kind.as_str() {
                "str" | "f32x4" => format!(" {0}={{{0}:?}}", f.name),
                k if list(k).is_some() || crate::word(k) || crate::optional(k).is_some() => {
                    format!(" {0}={{{0}:?}}", f.name)
                }
                _ => format!(" {0}={{{0}}}", f.name),
            })
            .collect();
        let pattern = if names.is_empty() {
            String::new()
        } else {
            format!(" {{ {} }}", names.join(", "))
        };
        writeln!(
            s,
            "            Self::{}{pattern} => write!(f, \"{}{shown}\"),",
            pascal(&op.name),
            op.name
        )
        .unwrap();
    }
    s += "        }\n    }\n}\n";
    s
}

pub fn exposed(op: &Op, class: &str) -> bool {
    let manual = matches!(
        op.name.as_str(),
        "begin_compute_pass" | "end_compute_pass" | "begin_render_pass" | "end_render_pass"
    );
    !op.internal && !manual && op.scope.split(' ').any(|s| s == class)
}

fn param(f: &Field) -> String {
    match list(&f.kind) {
        Some(item) => format!("{}: &[crate::{item}]", f.name),
        None => format!("{}: {}", f.name, rust_type(&f.kind).replace("'a ", "")),
    }
}

pub fn methods(ops: &[Op]) -> String {
    let mut s = String::new();
    for (class, kind) in [
        ("outside", "crate::CommandBuffer<'_>"),
        ("compute", "crate::ComputePass<'_, '_>"),
        ("render", "crate::RenderPass<'_, '_>"),
    ] {
        writeln!(s, "impl {kind} {{").unwrap();
        for op in ops.iter().filter(|o| exposed(o, class)) {
            s += &gate(op, "    ");
            let params: Vec<_> = match &op.info {
                Some(info) => vec![format!("info: &crate::{info}")],
                None => op.fields.iter().map(param).collect(),
            };
            let owner = if op.info.is_some() { "info." } else { "" };
            let values: Vec<_> = op
                .fields
                .iter()
                .map(|f| match owner {
                    "" => f.name.clone(),
                    _ => format!("{}: {owner}{}", f.name, f.name),
                })
                .collect();
            let command = if op.fields.is_empty() {
                format!("crate::command::Command::{}", pascal(&op.name))
            } else {
                format!(
                    "crate::command::Command::{} {{\n            {},\n        }}",
                    pascal(&op.name),
                    values.join(",\n            ")
                )
            };
            let mut list = vec!["&mut self".to_string()];
            list.extend(params);
            if op.fails {
                writeln!(
                    s,
                    "    pub fn {}({}) -> Result<(), crate::Error> {{\n        self.recording.record({command})\n    }}\n",
                    op.name,
                    list.join(", ")
                )
                .unwrap();
            } else {
                writeln!(
                    s,
                    "    pub fn {}({}) {{\n        self.recording.run({command})\n    }}\n",
                    op.name,
                    list.join(", ")
                )
                .unwrap();
            }
        }
        s.truncate(s.len() - 1);
        s += "}\n\n";
    }
    s
}

const CLASSES: [(&str, &str); 3] = [
    ("outside", "CommandBuffer"),
    ("compute", "ComputePass"),
    ("render", "RenderPass"),
];

fn jni_params(f: &Field) -> Vec<(String, &'static str, &'static str)> {
    let n = crate::camel(&f.name);
    match f.kind.as_str() {
        "u32" | "i32" | "u8" => vec![(n, "int", "jint")],
        k if crate::word(k) => vec![(n, "int", "jint")],
        "u64" => vec![(n, "long", "jlong")],
        "f32" => vec![(n, "float", "jfloat")],
        "f32x4" => (0..4)
            .map(|i| (format!("{n}{i}"), "float", "jfloat"))
            .collect(),
        "bool" => vec![(n, "boolean", "jboolean")],
        k if crate::handle(k) => vec![(n, "long", "jlong")],
        _ => vec![
            (n.clone(), "long", "jlong"),
            (format!("{n}Length"), "int", "jint"),
        ],
    }
}

fn snake(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_uppercase() {
            s.push('_');
            s.push(c.to_ascii_lowercase());
        } else {
            s.push(c);
        }
    }
    s
}

fn rust_value(f: &Field) -> String {
    let n = snake(&crate::camel(&f.name));
    match f.kind.as_str() {
        "u32" => format!("{n} as u32"),
        "u8" => format!("{n} as u8"),
        "i32" | "f32" => n,
        "f32x4" => format!("[{n}0, {n}1, {n}2, {n}3]"),
        "u64" => format!("{n} as u64"),
        "bool" => format!("{n} != 0"),
        "str" => format!("crate::java::str({n}, {n}_length as i64)"),
        k if crate::handle(k) => format!("crate::{k}::from_raw({n} as u64)"),
        k if crate::word(k) => format!("crate::{k}::of({n} as u32)"),
        k => {
            let item = list(k).unwrap();
            format!(
                "&crate::wire::list(&mut crate::java::bytes({n}, {n}_length as i64), crate::{item}::read)"
            )
        }
    }
}

pub fn jni(ops: &[Op]) -> String {
    let mut s = String::new();
    for (scope, class) in CLASSES {
        let getter = snake(class).trim_start_matches('_').to_string();
        for op in ops.iter().filter(|o| exposed(o, scope)) {
            let mut params = vec![
                "mut env: JNIEnv<'local>".to_string(),
                "_: JClass".to_string(),
                "target: jlong".to_string(),
            ];
            for f in &op.fields {
                for (n, _, t) in jni_params(f) {
                    params.push(format!("{}: {t}", snake(&n)));
                }
            }
            let values: Vec<_> = op.fields.iter().map(rust_value).collect();
            let args = match &op.info {
                Some(info) => {
                    let fields: Vec<_> = op
                        .fields
                        .iter()
                        .zip(&values)
                        .map(|(f, v)| {
                            if *v == f.name {
                                v.clone()
                            } else {
                                format!("{}: {v}", f.name)
                            }
                        })
                        .collect();
                    format!("&crate::{info} {{ {} }}", fields.join(", "))
                }
                None => values.join(", "),
            };
            let call = format!("crate::java::{getter}(target).{}({args})", op.name);
            let body = if op.fails {
                call
            } else {
                format!("{call};\n        Ok(())")
            };
            s += "\n";
            if let Some(c) = cfg(op.backend.as_deref()) {
                writeln!(s, "#[cfg({c})]").unwrap();
            }
            writeln!(
                s,
                "#[unsafe(no_mangle)]\npub extern \"system\" fn Java_dev_dov_mk_{class}_n{}<'local>({}) {{\n    crate::java::call(&mut env, |_| {{\n        {body}\n    }})\n}}",
                pascal(&op.name),
                params.join(", ")
            )
            .unwrap();
        }
    }
    s
}

pub fn java(ops: &[Op], scope: &str, class: &str) -> String {
    let mut s = String::from(
        "package dev.dov.mk;\n\nimport static org.lwjgl.system.MemoryUtil.*;\n\nimport java.nio.charset.StandardCharsets;\n\n",
    );
    s += &match class {
        "CommandBuffer" => include_str!("command_buffer.java").to_string(),
        _ => include_str!("pass.java").replace("NAME", class),
    };
    for op in ops.iter().filter(|o| exposed(o, scope)) {
        let params: Vec<_> = match &op.info {
            Some(info) => vec![format!("{info} info")],
            None => op
                .fields
                .iter()
                .map(|f| {
                    let t = match f.kind.as_str() {
                        "u32" | "i32" | "u8" => "int".to_string(),
                        k if crate::flags(k) => "int".into(),
                        k if crate::enumerated(k) => k.into(),
                        "u64" => "long".into(),
                        "f32" => "float".into(),
                        "f32x4" => "float[]".into(),
                        "bool" => "boolean".into(),
                        "str" => "String".into(),
                        k if crate::handle(k) => k.into(),
                        k => format!("{}[]", list(k).unwrap()),
                    };
                    format!("{t} {}", crate::camel(&f.name))
                })
                .collect(),
        };
        writeln!(
            s,
            "\n    public void {}({}) {{",
            crate::camel(&op.name),
            params.join(", ")
        )
        .unwrap();
        let owner = if op.info.is_some() { "info." } else { "" };
        let mut args = vec!["handle()".to_string()];
        let mut arena = false;
        let mut body = String::new();
        for f in &op.fields {
            let n = format!("{owner}{}", crate::camel(&f.name));
            match f.kind.as_str() {
                k if crate::handle(k) => args.push(format!("{k}.raw({n})")),
                "u8" => args.push(format!("{n} & 0xff")),
                "f32x4" => args.extend((0..4).map(|i| format!("{n}[{i}]"))),
                k if crate::enumerated(k) => args.push(format!("{n}.value")),
                "str" => {
                    arena = true;
                    writeln!(
                        body,
                        "            var encoded = {n}.getBytes(StandardCharsets.UTF_8);"
                    )
                    .unwrap();
                    args.push("arena.bytes(encoded)".into());
                    args.push("encoded.length".into());
                }
                k if list(k).is_some() => {
                    arena = true;
                    crate::infos::list_size(&mut body, k, &n, "            ");
                    writeln!(
                        body,
                        "            var list = arena.alloc({n}Size);\n            var at = list;"
                    )
                    .unwrap();
                    crate::infos::list_put(&mut body, k, &n, "at", "            ").unwrap();
                    args.push("list".into());
                    args.push(format!("{n}Size"));
                }
                _ => args.push(n),
            }
        }
        let call = format!("n{}({});", pascal(&op.name), args.join(", "));
        if arena {
            writeln!(s, "        try (var arena = new Arena()) {{").unwrap();
            s.push_str(&body);
            writeln!(s, "            {call}\n        }}").unwrap();
        } else {
            s.push_str(&body);
            writeln!(s, "        {call}").unwrap();
        }
        s += "    }\n";
        let mut natives = vec!["long target".to_string()];
        for f in &op.fields {
            for (n, t, _) in jni_params(f) {
                natives.push(format!("{t} {n}"));
            }
        }
        writeln!(
            s,
            "\n    private static native void n{}({});",
            pascal(&op.name),
            natives.join(", ")
        )
        .unwrap();
    }
    s += "}\n";
    s
}
