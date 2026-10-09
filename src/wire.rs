fn take<'a>(b: &mut &'a [u8], n: usize) -> &'a [u8] {
    let (head, tail) = b.split_at(n);
    *b = tail;
    head
}

pub fn u32(b: &mut &[u8]) -> u32 {
    u32::from_le_bytes(take(b, 4).try_into().unwrap())
}

pub fn u64(b: &mut &[u8]) -> u64 {
    u64::from_le_bytes(take(b, 8).try_into().unwrap())
}

pub fn f32(b: &mut &[u8]) -> f32 {
    f32::from_le_bytes(take(b, 4).try_into().unwrap())
}

pub fn f32x4(b: &mut &[u8]) -> [f32; 4] {
    [f32(b), f32(b), f32(b), f32(b)]
}

pub fn bool(b: &mut &[u8]) -> bool {
    u32(b) != 0
}

pub fn bytes<'a>(b: &mut &[u8]) -> &'a [u8] {
    let address = u64(b) as i64;
    crate::java::bytes(address, u64(b) as i64)
}

pub fn str<'a>(b: &mut &'a [u8]) -> &'a str {
    let n = u32(b) as usize;
    std::str::from_utf8(take(b, n)).unwrap()
}

pub fn list<'a, T>(b: &mut &'a [u8], f: impl Fn(&mut &'a [u8]) -> T) -> Vec<T> {
    (0..u32(b)).map(|_| f(b)).collect()
}

pub fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_f32x4(out: &mut Vec<u8>, v: [f32; 4]) {
    for x in v {
        put_f32(out, x);
    }
}

pub fn put_bool(out: &mut Vec<u8>, v: bool) {
    put_u32(out, v as u32);
}
