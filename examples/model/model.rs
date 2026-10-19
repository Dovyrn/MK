use png::Transformations;
use std::collections::HashMap;
use std::path::Path;

pub struct Part {
    pub material: usize,
    pub first: u32,
    pub count: u32,
}

pub struct Material {
    pub name: String,
    pub levels: Vec<Vec<[u8; 4]>>,
    pub width: u32,
    pub height: u32,
}

pub struct Model {
    pub vertices: Vec<[f32; 8]>,
    pub indices: Vec<u32>,
    pub parts: Vec<Part>,
    pub materials: Vec<Material>,
}

struct Entry {
    name: String,
    color: [f32; 3],
    diffuse: Option<String>,
    mask: Option<String>,
}

fn numbers(words: std::str::SplitWhitespace) -> [f32; 3] {
    let mut v = [0.0; 3];
    for (slot, word) in v.iter_mut().zip(words) {
        *slot = word.parse().unwrap();
    }
    v
}

fn decode(path: &Path) -> (Vec<[u8; 4]>, u32, u32) {
    let mut decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut bytes).unwrap();
    let channels = info.color_type.samples();
    let texels = bytes[..info.buffer_size()]
        .chunks(channels)
        .map(|c| match channels {
            1 => [c[0], c[0], c[0], 255],
            2 => [c[0], c[0], c[0], c[1]],
            3 => [c[0], c[1], c[2], 255],
            _ => [c[0], c[1], c[2], c[3]],
        })
        .collect();
    (texels, info.width, info.height)
}

fn halve(texels: &[[u8; 4]], width: u32, height: u32) -> Vec<[u8; 4]> {
    let (w, h) = ((width / 2).max(1), (height / 2).max(1));
    let at = |x: u32, y: u32| texels[(y.min(height - 1) * width + x.min(width - 1)) as usize];
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w * 2, i / w * 2);
            let quad = [at(x, y), at(x + 1, y), at(x, y + 1), at(x + 1, y + 1)];
            std::array::from_fn(|k| (quad.iter().map(|t| t[k] as u32).sum::<u32>() / 4) as u8)
        })
        .collect()
}

fn material(dir: &Path, entry: Entry) -> Material {
    let file = |name: &str| dir.join(name.replace('\\', "/"));
    let (mut texels, width, height) = match &entry.diffuse {
        Some(d) => decode(&file(d)),
        None => {
            let [r, g, b] = entry.color.map(|v| (v * 255.0) as u8);
            (vec![[r, g, b, 255]], 1, 1)
        }
    };
    if let Some(m) = &entry.mask {
        let (alpha, w, h) = decode(&file(m));
        assert_eq!(
            (w, h),
            (width, height),
            "{m} differs in size from its diffuse"
        );
        for (t, a) in texels.iter_mut().zip(alpha) {
            t[3] = a[0];
        }
    }
    let mut levels = vec![texels];
    let (mut w, mut h) = (width, height);
    while w > 1 || h > 1 {
        levels.push(halve(levels.last().unwrap(), w, h));
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
    }
    Material {
        name: entry.name,
        levels,
        width,
        height,
    }
}

fn materials(path: &Path) -> Vec<Material> {
    let mut entries: Vec<Entry> = Vec::new();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let mut words = line.split_whitespace();
        let key = words.next();
        let rest = words.collect::<Vec<_>>().join(" ");
        if key == Some("newmtl") {
            entries.push(Entry {
                name: rest,
                color: [1.0; 3],
                diffuse: None,
                mask: None,
            });
            continue;
        }
        let Some(e) = entries.last_mut() else {
            continue;
        };
        match key {
            Some("Kd") => {
                let v: Vec<f32> = rest.split(' ').map(|w| w.parse().unwrap()).collect();
                e.color = [v[0], v[1], v[2]];
            }
            Some("map_Kd") => e.diffuse = Some(rest),
            Some("map_d") => e.mask = Some(rest),
            _ => {}
        }
    }
    let dir = path.parent().unwrap();
    entries.into_iter().map(|e| material(dir, e)).collect()
}

impl Model {
    pub fn load(path: &str) -> Self {
        let path = Path::new(path);
        let text = std::fs::read_to_string(path).unwrap();
        let (mut positions, mut normals, mut uvs) = (Vec::new(), Vec::new(), Vec::new());
        let mut seen = HashMap::new();
        let mut model = Model {
            vertices: Vec::new(),
            indices: Vec::new(),
            parts: Vec::new(),
            materials: Vec::new(),
        };
        for line in text.lines() {
            let mut words = line.split_whitespace();
            match words.next() {
                Some("v") => positions.push(numbers(words)),
                Some("vn") => normals.push(numbers(words)),
                Some("vt") => uvs.push(numbers(words)),
                Some("mtllib") => {
                    let name = words.collect::<Vec<_>>().join(" ");
                    model
                        .materials
                        .extend(materials(&path.with_file_name(name)));
                }
                Some("usemtl") => {
                    let name = words.next().unwrap();
                    let material = model.materials.iter().position(|m| m.name == name).unwrap();
                    model.parts.push(Part {
                        material,
                        first: model.indices.len() as u32,
                        count: 0,
                    });
                }
                Some("f") => {
                    let corners: Vec<u32> = words
                        .map(|word| {
                            let mut parts = word.split('/');
                            let p: usize = parts.next().unwrap().parse().unwrap();
                            let mut index = || parts.next().map_or(0, |n| n.parse().unwrap_or(0));
                            let (t, n): (usize, usize) = (index(), index());
                            *seen.entry((p, t, n)).or_insert_with(|| {
                                let normal = normals.get(n.wrapping_sub(1)).copied();
                                let uv = uvs.get(t.wrapping_sub(1)).copied();
                                let [x, y, z] = positions[p - 1];
                                let [a, b, c] = normal.unwrap_or([0.0, 1.0, 0.0]);
                                let [u, v, _] = uv.unwrap_or([0.0; 3]);
                                model.vertices.push([x, y, z, a, b, c, u, 1.0 - v]);
                                model.vertices.len() as u32 - 1
                            })
                        })
                        .collect();
                    for i in 1..corners.len() - 1 {
                        model
                            .indices
                            .extend([corners[0], corners[i], corners[i + 1]]);
                    }
                    if let Some(part) = model.parts.last_mut() {
                        part.count = model.indices.len() as u32 - part.first;
                    }
                }
                _ => {}
            }
        }
        model.fit();
        model
    }

    fn fit(&mut self) {
        let mut low = [f32::MAX; 3];
        let mut high = [f32::MIN; 3];
        for v in &self.vertices {
            for i in 0..3 {
                low[i] = low[i].min(v[i]);
                high[i] = high[i].max(v[i]);
            }
        }
        let center: [f32; 3] = std::array::from_fn(|i| (low[i] + high[i]) / 2.0);
        let size = (0..3).map(|i| high[i] - low[i]).fold(0.0, f32::max);
        for v in &mut self.vertices {
            for i in 0..3 {
                v[i] = (v[i] - center[i]) * 2.0 / size;
            }
        }
    }
}
