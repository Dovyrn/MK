use crate::gl::cache::{Cache, key};
use crate::gl::extra::Extra;
use crate::register;
use glow::HasContext;

pub struct Program {
    pub raw: glow::Program,
    pub pairs: Vec<(u32, Option<u32>)>,
    pub base: Option<glow::UniformLocation>,
    pub instance: Option<glow::UniformLocation>,
}

const STORAGE_BLOCK: u32 = 0x92E6;

const INVALID: u32 = u32::MAX;

fn section<'a>(text: &'a str, entry: &str) -> Result<&'a str, String> {
    let marker = format!("//@ entry {entry}\n");
    let start = text
        .find(&marker)
        .ok_or(format!("the GL code has no entry {entry}"))?
        + marker.len();
    let end = text[start..]
        .find("//@ entry ")
        .map_or(text.len(), |i| start + i);
    Ok(&text[start..end])
}

unsafe fn link(
    gl: &glow::Context,
    raw: glow::Program,
    text: &str,
    stages: &[(u32, &str)],
) -> Result<(), String> {
    unsafe {
        let mut shaders = Vec::new();
        let mut failure = None;
        for &(kind, entry) in stages {
            let shader = gl.create_shader(kind)?;
            shaders.push(shader);
            match section(text, entry) {
                Ok(s) => gl.shader_source(shader, &s[..s.find("//@ ").unwrap_or(s.len())]),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                failure = Some(format!("{entry}: {}", gl.get_shader_info_log(shader)));
                break;
            }
            gl.attach_shader(raw, shader);
        }
        if failure.is_none() {
            gl.link_program(raw);
            if !gl.get_program_link_status(raw) {
                failure = Some(gl.get_program_info_log(raw));
            }
        }
        for shader in shaders {
            gl.delete_shader(shader);
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Program {
    pub fn new(
        gl: &glow::Context,
        extra: &Extra,
        code: &[u8],
        stages: &[(u32, &str)],
        cache: Option<&mut Cache>,
    ) -> Result<Self, String> {
        let text = std::str::from_utf8(code).map_err(|e| e.to_string())?;
        let key = key(code, stages);
        unsafe {
            let raw = gl.create_program()?;
            let loaded = match cache.as_ref().and_then(|c| c.programs.get(&key)) {
                Some(binary) => {
                    gl.program_binary(raw, binary);
                    gl.get_program_link_status(raw)
                }
                None => false,
            };
            if !loaded {
                if cache.is_some() {
                    gl.program_binary_retrievable_hint(raw, true);
                }
                if let Err(e) = link(gl, raw, text, stages) {
                    gl.delete_program(raw);
                    return Err(e);
                }
                if let Some(c) = cache
                    && let Some(binary) = gl.get_program_binary(raw)
                {
                    c.programs.insert(key, binary);
                }
            }
            let mut pairs = Vec::new();
            let old = gl.get_parameter_i32(glow::CURRENT_PROGRAM) as u32;
            gl.use_program(Some(raw));
            let lines = stages
                .iter()
                .filter_map(|&(_, entry)| section(text, entry).ok())
                .flat_map(str::lines)
                .filter_map(|l| l.strip_prefix("//@ "));
            let mut seen = Vec::new();
            for line in lines {
                if seen.contains(&line) {
                    continue;
                }
                seen.push(line);
                let words: Vec<&str> = line.split_whitespace().collect();
                let number = |i: usize| words[i].parse::<u32>().unwrap();
                match words[0] {
                    "uniform" => {
                        if let Some(i) = gl.get_uniform_block_index(raw, words[1]) {
                            gl.uniform_block_binding(raw, i, number(2));
                        }
                    }
                    "storage" => {
                        let Some(find) = extra.resource else {
                            continue;
                        };
                        let name = std::ffi::CString::new(words[1]).unwrap();
                        let i = find(raw.0.get(), STORAGE_BLOCK, name.as_ptr());
                        if i != INVALID {
                            gl.shader_storage_block_binding(raw, i, number(2) - register::STORAGE);
                        }
                    }
                    "image" => {
                        if let Some(l) = gl.get_uniform_location(raw, words[1]) {
                            gl.uniform_1_i32(Some(&l), (number(2) - register::IMAGE) as i32);
                        }
                    }
                    "texture" => {
                        if let Some(l) = gl.get_uniform_location(raw, words[1]) {
                            gl.uniform_1_i32(Some(&l), pairs.len() as i32);
                            pairs.push((number(2), words.get(3).map(|_| number(3))));
                        }
                    }
                    _ => {}
                }
            }
            gl.use_program(std::num::NonZeroU32::new(old).map(glow::NativeProgram));
            Ok(Self {
                raw,
                pairs,
                base: gl.get_uniform_location(raw, "SPIRV_Cross_BaseVertex"),
                instance: gl.get_uniform_location(raw, "SPIRV_Cross_BaseInstance"),
            })
        }
    }
}
