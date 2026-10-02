use mk::{Command, Commands, TextureBarrier, Viewport};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("path to the Java-recorded stream");
    let java = std::fs::read(path).unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.bind_pipeline(7);
    c.push_constants(16, &[1, 2, 3]);
    c.dispatch(1, 2, 3);
    c.dispatch_indirect(4, 1 << 40);
    c.copy_buffer(1, 2, 3, 4, 5);
    c.fill_buffer(9, 8, 7, 0xab);
    c.barrier(1, 2, 3, 4);
    c.begin_label("frame");
    c.end_label();
    c.end_compute();
    c.set_viewport(&Viewport {
        x: 0.5,
        y: 1.5,
        width: 64.0,
        height: 32.0,
        ..Default::default()
    });
    c.draw_indexed(3, 1, 0, -5, 2);
    c.texture_barrier(&TextureBarrier {
        texture: 1,
        src_stage: 2,
        src_access: 3,
        dst_stage: 4,
        dst_access: 5,
        before: 6,
        after: 7,
        ..Default::default()
    });
    assert_eq!(
        c.bytes(),
        &java[..],
        "\nrust:\n{}java:\n{}",
        Command::dump(c.bytes()),
        Command::dump(&java)
    );
    print!("{}", Command::dump(&java));
    println!("java and rust streams are identical ({} bytes)", java.len());
}
