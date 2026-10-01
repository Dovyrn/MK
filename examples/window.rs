#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, RenderInfo, SurfaceInfo, VertexAttribute, VertexBuffer, access,
    format, layout, load, memory, stage, store,
};
#[cfg(not(feature = "gl"))]
use winit::application::ApplicationHandler;
#[cfg(not(feature = "gl"))]
use winit::event::WindowEvent;
#[cfg(not(feature = "gl"))]
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(not(feature = "gl"))]
use winit::window::{Window, WindowId};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
struct State {
    window: Window,
    device: Device,
    surface: u32,
    pipeline: u32,
    vertices: u32,
    event: u32,
    frame: u64,
    size: (u32, u32),
}

#[cfg(not(feature = "gl"))]
#[derive(Default)]
struct App {
    state: Option<State>,
}

#[cfg(not(feature = "gl"))]
impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let window = el
            .create_window(Window::default_attributes().with_title("mk"))
            .unwrap();
        let size = window.inner_size();
        let mut device = Device::new(1).unwrap();
        let surface = device
            .create_surface(
                &window,
                &SurfaceInfo {
                    width: size.width,
                    height: size.height,
                    ..Default::default()
                },
            )
            .unwrap();
        let colors = [ColorTarget {
            format: format::BGRA8_UNORM,
            ..Default::default()
        }];
        let pipeline = device
            .create_render_pipeline(&RenderInfo {
                code: CODE,
                vertex: "vert",
                fragment: "frag",
                colors: &colors,
                buffers: &[VertexBuffer {
                    stride: 20,
                    instance: false,
                }],
                attributes: &[
                    VertexAttribute {
                        location: 0,
                        buffer: 0,
                        format: format::RG32_FLOAT,
                        offset: 0,
                    },
                    VertexAttribute {
                        location: 1,
                        buffer: 0,
                        format: format::RGB32_FLOAT,
                        offset: 8,
                    },
                ],
                ..Default::default()
            })
            .unwrap();
        let data: [[f32; 5]; 3] = [
            [-0.5, -0.5, 1.0, 0.0, 0.0],
            [0.5, -0.5, 0.0, 1.0, 0.0],
            [0.0, 0.5, 0.0, 0.0, 1.0],
        ];
        let vertices = device
            .create_buffer(size_of_val(&data) as u64, memory::UPLOAD)
            .unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr().cast(),
                device.map(vertices),
                size_of_val(&data),
            );
        }
        let event = device.create_event().unwrap();
        window.request_redraw();
        self.state = Some(State {
            window,
            device,
            surface,
            pipeline,
            vertices,
            event,
            frame: 0,
            size: (size.width, size.height),
        });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(s) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                s.device.wait(s.event, s.frame, u64::MAX);
                s.device
                    .resize_surface(s.surface, size.width, size.height)
                    .unwrap();
                s.size = (size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                s.device.wait(s.event, s.frame, u64::MAX);
                let target = s.device.acquire(0, s.surface).unwrap();
                let t = s.frame as f32 / 60.0;
                let all = !0;
                let mut c = Commands::default();
                c.texture_barrier(
                    target,
                    0,
                    0,
                    stage::COLOR,
                    access::COLOR_WRITE,
                    layout::UNDEFINED,
                    layout::COLOR,
                    0,
                    all,
                    0,
                    all,
                );
                c.color_attachment(
                    0,
                    target,
                    layout::COLOR,
                    load::CLEAR,
                    store::STORE,
                    t.sin() * 0.5 + 0.5,
                    0.2,
                    0.3,
                    1.0,
                    !0,
                    0,
                );
                c.begin_render(s.size.0, s.size.1, 1);
                c.bind_render_pipeline(s.pipeline);
                c.bind_vertex_buffer(0, s.vertices, 0);
                c.push_constants(0, &0.5f32.to_le_bytes());
                c.draw(3, 1, 0, 0);
                c.end_render();
                c.texture_barrier(
                    target,
                    stage::COLOR,
                    access::COLOR_WRITE,
                    0,
                    0,
                    layout::COLOR,
                    layout::PRESENT,
                    0,
                    all,
                    0,
                    all,
                );
                s.frame += 1;
                s.device.submit(0, &c, &[], &[(s.event, s.frame)]).unwrap();
                s.device.present(0, s.surface).unwrap();
                if s.frame == 120 {
                    s.device.wait(s.event, s.frame, u64::MAX);
                    println!("{} frames presented", s.frame);
                    el.exit();
                } else {
                    s.window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

#[cfg(not(feature = "gl"))]
fn main() {
    let el = EventLoop::new().unwrap();
    el.run_app(&mut App::default()).unwrap();
}
