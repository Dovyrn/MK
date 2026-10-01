use mk::{Command, Commands};

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
    c.set_viewport(0.5, 1.5, 64.0, 32.0, 0.0, 1.0);
    c.draw_indexed(3, 1, 0, -5, 2);
    c.texture_barrier(1, 2, 3, 4, 5, 6, 7, 0, !0, 0, !0);
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
