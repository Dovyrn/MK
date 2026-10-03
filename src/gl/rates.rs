use crate::combiner;
use crate::gl::extra::cast;
use std::ffi::c_void;

type Bind = unsafe extern "system" fn(u32);
type Palette = unsafe extern "system" fn(u32, u32, i32, *const u32);
type Entry = unsafe extern "system" fn(u32, u32, *mut u32);

pub const ENABLE: u32 = 0x9563;

pub const BINDING: u32 = 0x955B;

pub const TEXEL: [u32; 2] = [0x955C, 0x955D];

pub const ENTRIES: usize = 16;

const SIZES: [(u32, u32, u32); 7] = [
    (4, 4, 0x956B),
    (4, 2, 0x956A),
    (2, 4, 0x9569),
    (2, 2, 0x9568),
    (2, 1, 0x9567),
    (1, 2, 0x9566),
    (1, 1, 0x9565),
];

pub struct Rates {
    pub bind: Bind,
    pub palette: Palette,
    pub entry: Entry,
}

fn combine(op: u32, (aw, ah): (u32, u32), (bw, bh): (u32, u32)) -> (u32, u32) {
    match op {
        combiner::KEEP => (aw, ah),
        combiner::REPLACE => (bw, bh),
        combiner::MIN => (aw.min(bw), ah.min(bh)),
        combiner::MAX => (aw.max(bw), ah.max(bh)),
        combiner::MUL => (aw * bw, ah * bh),
        op => panic!("unknown combiner {op}"),
    }
}

pub fn palette([width, height, primitive, attachment]: [u32; 4]) -> [u32; ENTRIES] {
    let base = combine(primitive, (width, height), (1, 1));
    std::array::from_fn(|v| {
        let texel = (1 << (v >> 2), 1 << (v & 3));
        let (w, h) = combine(attachment, base, texel);
        let &(_, _, rate) = SIZES.iter().find(|&&(x, y, _)| x <= w && y <= h).unwrap();
        rate
    })
}

impl Rates {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                bind: cast(get("glBindShadingRateImageNV")?),
                palette: cast(get("glShadingRateImagePaletteNV")?),
                entry: cast(get("glGetShadingRateImagePaletteNV")?),
            })
        }
    }
}
