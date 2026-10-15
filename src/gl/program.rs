use crate::Error;
use crate::register;
use glow::HasContext;

pub struct Program {
    pub raw: glow::Program,
    pub pairs: Vec<(u32, Option<u32>)>,
    pub instance: Option<glow::UniformLocation>,
}

fn entry_section<'a>(text: &'a str, entry: &str) -> Result<&'a str, Error> {
    let marker = format!("//@ entry {entry}\n");
    let Some(start) = text.find(&marker) else {
        let list: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("//@ entry "))
            .collect();
        return Err(Error::Shader(format!(
            "no entry {entry} in the code; it has {}",
            list.join(", ")
        )));
    };
    let start = start + marker.len();
    let end = text[start..]
        .find("//@ entry ")
        .map_or(text.len(), |i| start + i);
    Ok(&text[start..end])
}

pub unsafe fn link(
    gl: &glow::Context,
    raw: glow::Program,
    text: &str,
    stages: &[(u32, &str)],
) -> Result<(), Error> {
    unsafe {
        let mut shaders = Vec::new();
        let mut failure = None;
        for &(kind, entry) in stages {
            let shader = gl.create_shader(kind).map_err(Error::Driver)?;
            shaders.push(shader);
            match entry_section(text, entry) {
                Ok(s) => gl.shader_source(shader, &s[..s.find("//@ ").unwrap_or(s.len())]),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                failure = Some(Error::Shader(format!(
                    "{entry}: {}",
                    gl.get_shader_info_log(shader)
                )));
                break;
            }
            gl.attach_shader(raw, shader);
        }
        if failure.is_none() {
            gl.link_program(raw);
            if !gl.get_program_link_status(raw) {
                failure = Some(Error::Shader(gl.get_program_info_log(raw)));
            }
        }
        for shader in shaders {
            gl.delete_shader(shader);
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Program {
    pub fn new(gl: &glow::Context, code: &[u8], stages: &[(u32, &str)]) -> Result<Self, Error> {
        let text = std::str::from_utf8(code).map_err(|e| Error::Shader(e.to_string()))?;
        unsafe {
            let raw = gl.create_program().map_err(Error::Driver)?;
            if let Err(e) = link(gl, raw, text, stages) {
                gl.delete_program(raw);
                return Err(e);
            }
            let mut pairs = Vec::new();
            let old = gl.get_parameter_i32(glow::CURRENT_PROGRAM) as u32;
            gl.use_program(Some(raw));
            let lines = stages
                .iter()
                .filter_map(|&(_, entry)| entry_section(text, entry).ok())
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
                        if let Some(i) = gl.get_shader_storage_block_index(raw, words[1]) {
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
                instance: gl.get_uniform_location(raw, "mk_BaseInstance"),
            })
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.raw) };
    }
}
