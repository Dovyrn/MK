use crate::Triangles;

pub struct AccelInfo<'a> {
    pub triangles: &'a [Triangles],
    pub instances: u32,
    pub instance_offset: u64,
    pub instance_count: u32,
    pub flags: u32,
}

impl Default for AccelInfo<'_> {
    fn default() -> Self {
        Self {
            triangles: &[],
            instances: !0,
            instance_offset: 0,
            instance_count: 0,
            flags: 0,
        }
    }
}
