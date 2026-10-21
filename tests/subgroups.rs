use mk::{BufferUsage, ComputePipelineInfo, Memory, PipelineStages, SubgroupOp};

use crate::host;

pub fn test() {
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.capabilities();
    let need = SubgroupOp::BASIC
        | SubgroupOp::VOTE
        | SubgroupOp::ARITHMETIC
        | SubgroupOp::BALLOT
        | SubgroupOp::SHUFFLE
        | SubgroupOp::QUAD;
    if !caps.compute_shaders || !caps.storage_sets || caps.subgroup_ops & need != need {
        println!("no subgroup ops in compute: {:#x}", caps.subgroup_ops);
        return;
    }
    if !caps.subgroup_stages.contains(PipelineStages::COMPUTE) {
        println!("no subgroup ops in compute shaders");
        return;
    }
    let buffer = device
        .create_buffer(64 * 11 * 4, Memory::Readback, BufferUsage::STORAGE, "")
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer.into()]);
    device.commit_residency(set);
    let layout = host::layout(&device, &[mk::DescriptorType::StorageBuffer]);
    let bound = host::set(&device, layout, &[host::buffer(0, buffer, 0, 64 * 11 * 4)]);
    let sizes: Vec<u32> = (0..7)
        .map(|i| 1u32 << i)
        .filter(|n| caps.subgroup_size_mask & n != 0 && *n >= 4)
        .collect();
    let mut runs = vec![(0, false)];
    runs.extend(sizes.iter().map(|&n| (n, false)));
    let largest = 1u32 << (31 - caps.subgroup_size_mask.leading_zeros());
    if caps.full_subgroups && largest <= 64 {
        runs.push((0, true));
    }
    let mut seen = Vec::new();
    for (value, &(size, full)) in (1..).zip(&runs) {
        let pipeline = device
            .create_compute_pipeline(&ComputePipelineInfo {
                bytecode: host::code!("subgroups"),
                entrypoint: "lanes",
                required_subgroup_size: size,
                require_full_subgroups: full,
                descriptor_set_layout: &[layout],
                ..Default::default()
            })
            .unwrap();
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let mut pass = c
            .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
            .unwrap();
        pass.bind_compute_pipeline(pipeline);
        pass.bind_descriptor_set(bound);
        pass.dispatch(1, 1, 1);
        pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::HOST);
        pass.end();
        c.end().unwrap();
        device.submit([c], &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let r = unsafe { device.map_buffer::<[u32; 11]>(buffer) };
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
