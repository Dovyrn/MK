use mk::{Buffer, BufferUsage, CommandPool, Device, Memory, PipelineStages, ResidencySet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::host;

fn copy(device: &Device, pool: CommandPool, set: ResidencySet, a: Buffer, b: Buffer) {
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.copy_buffer_to_buffer(a, 0, b, 0, 16);
    pass.end();
}

fn message(run: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(run)).expect_err("expected a panic");
    error.downcast_ref::<String>().cloned().unwrap_or_default()
}

pub fn test() {
    if !cfg!(feature = "validation") {
        println!("residency checks need the validation feature");
        return;
    }
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let a = device
        .create_buffer(16, Memory::Upload, BufferUsage::COPY_SRC, "")
        .unwrap();
    let b = device
        .create_buffer(16, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let set = device.create_residency_set().unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let missing = message(|| copy(&device, pool, set, a, b));
    device.add_residency(set, &[a.into(), b.into()]);
    let uncommitted = message(|| copy(&device, pool, set, a, b));
    device.commit_residency(set);
    let unused = message(|| {
        let mut c = device.begin_commands(pool).unwrap();
        let mut pass = c
            .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
            .unwrap();
        pass.copy_buffer_to_buffer(a, 0, b, 0, 16);
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
    unsafe { device.map_buffer::<u32>(a) }.copy_from_slice(&[7, 8, 9, 10]);
    device.unmap_buffer(a);
    device.attach_residency_set(0, set);
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let attached = message(|| device.destroy_residency_set(set));
    std::panic::set_hook(hook);
    assert!(
        attached.contains("detach_residency_set first"),
        "{attached}"
    );
    let mut c = device.begin_commands(pool).unwrap();
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.copy_buffer_to_buffer(a, 0, b, 0, 16);
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    let timeline = device.create_timeline().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap(), "the wait timed out");
    assert_eq!(unsafe { device.map_buffer::<u32>(b) }, [7, 8, 9, 10]);
    device.detach_residency_set(0, set);
    device.remove_residency(set, &[a.into(), b.into()]);
    device.commit_residency(set);
    device.destroy_buffer(a);
    device.destroy_buffer(b);
    println!(
        "residency panics on missing, uncommitted and unused sets and on destroying a held buffer; a queue-attached set covers a copy"
    );
}
