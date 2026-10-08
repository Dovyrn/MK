use mk::{ComputePipelineInfo, memory, stage, subgroup_op};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    let need = subgroup_op::BASIC
        | subgroup_op::VOTE
        | subgroup_op::ARITHMETIC
        | subgroup_op::BALLOT
        | subgroup_op::SHUFFLE
        | subgroup_op::QUAD;
    if !caps.compute || !caps.storage || caps.subgroup_ops & need != need {
        println!("no subgroup ops in compute: {:#x}", caps.subgroup_ops);
        return;
    }
    if caps.subgroup_stages & stage::COMPUTE == 0 {
        println!("no subgroup ops in compute shaders");
        return;
    }
    let buffer = device.create_buffer(64 * 11 * 4, memory::READBACK).unwrap();
    let timeline = device.create_timeline().unwrap();
    let sizes: Vec<u32> = (0..7)
        .map(|i| 1u32 << i)
        .filter(|n| caps.subgroup_sizes & n != 0 && *n >= 4)
        .collect();
    let mut runs = vec![(0, false)];
    runs.extend(sizes.iter().map(|&n| (n, false)));
    let largest = 1u32 << (31 - caps.subgroup_sizes.leading_zeros());
    if caps.full_subgroups && largest <= 64 {
        runs.push((0, true));
    }
    let mut seen = Vec::new();
    for (value, &(size, full)) in (1..).zip(&runs) {
        let pipeline = device
            .create_compute_pipeline(&ComputePipelineInfo {
                code: host::code!("subgroups"),
                entry: "lanes",
                subgroup: size,
                full,
            })
            .unwrap();
        let mut c = device.begin_commands(pool).unwrap();
        let mut pass = c.begin_compute_pass().unwrap();
        pass.bind_compute_pipeline(pipeline).unwrap();
        pass.bind_storage_buffer(0, buffer, 0, 64 * 11 * 4).unwrap();
        pass.dispatch(1, 1, 1).unwrap();
        pass.memory_barrier(stage::COMPUTE, stage::HOST);
        pass.end();
        device.submit(c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let r = unsafe {
            std::slice::from_raw_parts(device.map_buffer(buffer).cast::<[u32; 11]>(), 64)
        };
        let count = r[0][0];
        assert!(
            size == 0 || count == size,
            "asked for {size} lanes, got {count}"
        );
        assert!(64 % count == 0, "{count} lanes do not tile 64 threads");
        let even = (0..count.min(32)).step_by(2).fold(0u32, |m, l| m | 1 << l);
        for (i, t) in r.iter().enumerate() {
            let lane = t[1];
            let want = [
                count,
                lane,
                count,
                lane * lane.saturating_sub(1) / 2,
                even,
                5,
                1,
                lane ^ 1,
                1,
                count - 1,
                r[63 - i][1],
            ];
            assert_eq!(*t, want, "thread {i} with {count} lanes");
        }
        device.unmap_buffer(buffer);
        device.destroy_compute_pipeline(pipeline);
        seen.push(format!("{count}{}", if full { " full" } else { "" }));
    }
    device.destroy_timeline(timeline);
    println!(
        "sum, prefix, ballot, broadcast, shuffle, quad, vote and max match at {} lanes",
        seen.join(", ")
    );
}
