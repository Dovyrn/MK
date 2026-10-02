use std::io::Cursor;

const ENTRY: u32 = 15;
const VARIABLE: u32 = 59;
const DECORATE: u32 = 71;
const VERTEX: u32 = 0;
const INPUT: u32 = 1;
const LOCATION: u32 = 30;

fn instructions(words: &[u32]) -> impl Iterator<Item = (u32, &[u32])> {
    let mut rest = words.get(5..).unwrap_or_default();
    std::iter::from_fn(move || {
        let count = (rest.first()? >> 16) as usize;
        let args = rest.get(1..count)?;
        let op = rest[0] & 0xffff;
        rest = &rest[count..];
        Some((op, args))
    })
}

fn entries(words: &[u32]) -> impl Iterator<Item = (u32, String, &[u32])> {
    instructions(words)
        .filter(|(op, args)| *op == ENTRY && args.len() > 2)
        .map(|(_, args)| {
            let bytes: Vec<u8> = args[2..].iter().flat_map(|w| w.to_le_bytes()).collect();
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let name = String::from_utf8_lossy(&bytes[..end]).into_owned();
            let interface = args.get(3 + end / 4..).unwrap_or_default();
            (args[0], name, interface)
        })
}

pub fn read(code: &[u8], names: &[&str]) -> Result<Vec<u32>, String> {
    let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
    let mut list: Vec<String> = Vec::new();
    for (_, name, _) in entries(&words) {
        if !list.contains(&name) {
            list.push(name);
        }
    }
    if let Some(name) = names.iter().find(|&&n| !list.iter().any(|e| e == n)) {
        return Err(format!(
            "no entry {name} in the code; it has {}",
            list.join(", ")
        ));
    }
    Ok(words)
}

pub fn inputs(words: &[u32], entry: &str) -> Vec<u32> {
    let Some((_, _, interface)) =
        entries(words).find(|(model, name, _)| *model == VERTEX && name == entry)
    else {
        return Vec::new();
    };
    let mut inputs = Vec::new();
    let mut locations = Vec::new();
    for (op, args) in instructions(words) {
        if op == VARIABLE && args.len() > 2 && args[2] == INPUT && interface.contains(&args[1]) {
            inputs.push(args[1]);
        }
        if op == DECORATE && args.len() > 2 && args[1] == LOCATION {
            locations.push((args[0], args[2]));
        }
    }
    let mut list: Vec<u32> = locations
        .into_iter()
        .filter(|(id, _)| inputs.contains(id))
        .map(|(_, location)| location)
        .collect();
    list.sort();
    list
}
