use crate::{atomic, subgroup};
use std::io::Cursor;

const ENTRY: u32 = 15;
const MODE: u32 = 16;
const CAPABILITY: u32 = 17;
const LOCAL_SIZE: u32 = 17;
const VARIABLE: u32 = 59;
const DECORATE: u32 = 71;
const VERTEX: u32 = 0;
const INPUT: u32 = 1;
const LOCATION: u32 = 30;
const SET: u32 = 34;

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

fn entries(words: &[u32]) -> impl Iterator<Item = (u32, u32, String, &[u32])> {
    instructions(words)
        .filter(|(op, args)| *op == ENTRY && args.len() > 2)
        .map(|(_, args)| {
            let bytes: Vec<u8> = args[2..].iter().flat_map(|w| w.to_le_bytes()).collect();
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let name = String::from_utf8_lossy(&bytes[..end]).into_owned();
            let interface = args.get(3 + end / 4..).unwrap_or_default();
            (args[0], args[1], name, interface)
        })
}

pub fn read(code: &[u8], names: &[&str], slots: bool) -> Result<Vec<u32>, String> {
    let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
    let mut list: Vec<String> = Vec::new();
    for (_, _, name, _) in entries(&words) {
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
    let old = words.get(1).is_some_and(|&v| v < 0x10400);
    let used: Vec<u32> = entries(&words)
        .filter(|(_, _, name, _)| names.contains(&name.as_str()))
        .flat_map(|(_, _, _, interface)| interface.to_vec())
        .collect();
    let zero = instructions(&words).any(|(op, args)| {
        op == DECORATE
            && args.len() > 2
            && args[1] == SET
            && args[2] == 0
            && (old || used.contains(&args[0]))
    });
    if zero && !slots {
        return Err("slot bindings need caps.slots".into());
    }
    Ok(words)
}

pub fn subgroups(words: &[u32]) -> u32 {
    instructions(words)
        .filter(|(op, _)| *op == CAPABILITY)
        .map(|(_, args)| match args[0] {
            61 => subgroup::BASIC,
            62 => subgroup::VOTE,
            63 => subgroup::ARITHMETIC,
            64 => subgroup::BALLOT,
            65 => subgroup::SHUFFLE,
            66 => subgroup::RELATIVE,
            67 => subgroup::CLUSTERED,
            68 => subgroup::QUAD,
            6026 => subgroup::ROTATE,
            _ => 0,
        })
        .fold(0, |a, b| a | b)
}

pub fn atomics(words: &[u32]) -> u32 {
    let image = if declares(words, 5016) {
        atomic::IMAGE
    } else {
        0
    };
    image | if declares(words, 12) { atomic::ALL } else { 0 }
}

pub fn declares(words: &[u32], capability: u32) -> bool {
    instructions(words).any(|(op, args)| op == CAPABILITY && args[0] == capability)
}

pub fn width(words: &[u32], entry: &str) -> u32 {
    let Some((_, function, _, _)) = entries(words).find(|(_, _, name, _)| name == entry) else {
        return 0;
    };
    instructions(words)
        .find(|(op, args)| *op == MODE && args[0] == function && args[1] == LOCAL_SIZE)
        .map_or(0, |(_, args)| args[2])
}

pub fn inputs(words: &[u32], entry: &str) -> Vec<u32> {
    let Some((_, _, _, interface)) =
        entries(words).find(|(model, _, name, _)| *model == VERTEX && name == entry)
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
