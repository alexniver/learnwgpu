use bytemuck::{Pod, Zeroable};
use tracing::{info, Level};
use wgpu::{include_wgsl, Backends, Instance};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    pollster::block_on(run(event_loop, window));
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

fn vertex(pos: [f32; 3], color: [f32; 3]) -> Vertex {
    Vertex { pos, color }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        vertex([-1., -1., 0.], [1., 0., 0.]), // left bottom, red
        vertex([1., -1., 0.], [0., 1., 0.]),  // right bottom, green
        vertex([0., 1., 0.], [0., 0., 1.]),   // top, blue
    ];

    let indices = vec![0, 1, 2];

    (vertices, indices)
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();

    let instance = Instance::new(Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Fail to create device");

    let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let preferred_format = surface.get_supported_formats(&adapter)[0];

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0=>Float32x3, 1=>Float32x3],
    };

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[vertex_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(preferred_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: preferred_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
    };

    surface.configure(&device, &config);
    let (verticrs, indices) = create_vertices();

    let vertices_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertices Buffer"),
        contents: bytemuck::cast_slice(&verticrs),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let indices_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Indeices Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &shader, &pipeline_layout);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_texture()
                    .expect("Fail to request next swap chain texture");

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_vertex_buffer(0, vertices_buf.slice(..));
                    rpass.set_index_buffer(indices_buf.slice(..), wgpu::IndexFormat::Uint16);

                    // rpass.draw(0..3, 0..1);
                    rpass.draw_indexed(0..indices.len() as u32, 0, 0..1)
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::RedrawEventsCleared => window.request_redraw(),
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    WindowEvent::Resized(size) => {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(&device, &config);

                        window.request_redraw(); // for macos, need redraw when size change
                    }

                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        info!("exit");
                        *control_flow = ControlFlow::Exit
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
