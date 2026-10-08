use mk::{Buffer, CommandPool, Device, ResidencySet, memory};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[path = "support/host.rs"]
mod host;

fn copy(device: &Device, pool: CommandPool, set: ResidencySet, a: Buffer, b: Buffer) {
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_buffer(a, 0, b, 0, 16);
    pass.end();
}

fn message(run: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(run)).expect_err("expected a panic");
    error.downcast_ref::<String>().cloned().unwrap_or_default()
}

pub fn main() {
    if !cfg!(feature = "validation") {
        println!("residency checks need the validation feature");
        return;
    }
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let a = device.create_buffer(16, memory::UPLOAD).unwrap();
    let b = device.create_buffer(16, memory::READBACK).unwrap();
    let set = device.create_residency_set().unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let missing = message(|| copy(&device, pool, set, a, b));
    device.add_residency(set, &[a.into(), b.into()]);
    let uncommitted = message(|| copy(&device, pool, set, a, b));
    device.commit_residency(set);
    let unused = message(|| {
        let mut c = device.begin_commands(pool).unwrap();
        let mut pass = c.begin_compute_pass().unwrap();
        pass.copy_buffer(a, 0, b, 0, 16);
        pass.end();
    });
    copy(&device, pool, set, a, b);
    let held = message(|| device.destroy_buffer(a));
    std::panic::set_hook(hook);
    for (what, text, want) in [
        ("missing", &missing, "no committed residency set holds"),
        (
            "uncommitted",
            &uncommitted,
            "no committed residency set holds",
        ),
        ("unused", &unused, "before use_residency_set"),
        ("held", &held, "still holds"),
    ] {
        assert!(text.contains(want), "{what}: {text}");
    }
    device.remove_residency(set, &[a.into(), b.into()]);
    device.commit_residency(set);
    device.destroy_buffer(a);
    device.destroy_buffer(b);
    println!(
        "residency panics on missing, uncommitted and unused sets and on destroying a held buffer"
    );
}
