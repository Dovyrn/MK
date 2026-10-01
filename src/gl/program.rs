use crate::gl::extra::Extra;
use crate::slot;
use glow::HasContext;

pub struct Program {
    pub raw: glow::Program,
    pub pairs: Vec<(u32, u32)>,
}

const STORAGE_BLOCK: u32 = 0x92E6;

const INVALID: u32 = u32::MAX;

fn source<'a>(text: &'a str, entry: &str) -> Result<&'a str, String> {
    let marker = format!("//@ entry {entry}\n");
    let start = text
        .find(&marker)
        .ok_or(format!("the GL code has no entry {entry}"))?
        + marker.len();
    let end = text[start..].find("//@ ").map_or(text.len(), |i| start + i);
    Ok(&text[start..end])
}

impl Program {
    pub fn new(
        gl: &glow::Context,
        extra: &Extra,
        code: &[u8],
        stages: &[(u32, &str)],
    ) -> Result<Self, String> {
        let text = std::str::from_utf8(code).map_err(|e| e.to_string())?;
        unsafe {
            let raw = gl.create_program()?;
            let mut shaders = Vec::new();
            let mut failure = None;
            for &(kind, entry) in stages {
                let shader = gl.create_shader(kind)?;
                shaders.push(shader);
                match source(text, entry) {
                    Ok(s) => gl.shader_source(shader, s),
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
            if let Some(e) = failure {
                gl.delete_program(raw);
                return Err(e);
            }
            let mut pairs = Vec::new();
            gl.use_program(Some(raw));
            for line in text.lines().filter_map(|l| l.strip_prefix("//@ ")) {
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
                            gl.shader_storage_block_binding(raw, i, number(2) - slot::STORAGE);
                        }
                    }
                    "image" => {
                        if let Some(l) = gl.get_uniform_location(raw, words[1]) {
                            gl.uniform_1_i32(Some(&l), (number(2) - slot::IMAGE) as i32);
                        }
                    }
                    "texture" => {
                        if let Some(l) = gl.get_uniform_location(raw, words[1]) {
                            gl.uniform_1_i32(Some(&l), pairs.len() as i32);
                            pairs.push((number(2), number(3)));
                        }
                    }
                    _ => {}
                }
            }
            gl.use_program(None);
            Ok(Self { raw, pairs })
        }
    }
}
