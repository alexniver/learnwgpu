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
    tex_coord: [f32; 2],
}

fn vertex(pos: [f32; 3], tex_coord: [f32; 2]) -> Vertex {
    Vertex { pos, tex_coord }
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        vertex([-0.5, -0.5, 0.], [0., 1.]), // left bottom
        vertex([0.5, -0.5, 0.], [1., 1.]),  // right bottom
        vertex([0.5, 0.5, 0.], [1., 0.]),   // top right
        vertex([-0.5, 0.5, 0.], [0., 0.]),  // top left
    ];

    let indices = vec![
        0, 1, 3, // first triangle
        1, 2, 3, // second triangle
    ];

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

    // texture
    // let diffuse_bytes = include_bytes!("happy-tree.png");
    let diffuse_bytes = include_bytes!("spengebob.jpeg");

    let diffuse_img = image::load_from_memory(diffuse_bytes).unwrap();
    let diffuse_rgba = diffuse_img.to_rgba8();
    // let diffuse_rgba = diffuse_img.as_rgba8().unwrap();

    use image::GenericImageView;
    let dimensions = diffuse_img.dimensions();

    info!("-----------{:?}", dimensions);

    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };

    let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("diffuse_texture"),
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &diffuse_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &diffuse_rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
            rows_per_image: std::num::NonZeroU32::new(dimensions.1),
        },
        texture_size,
    );

    let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("texture sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

    let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("diffuse_bind_group"),
        layout: &texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
            },
        ],
    });

    let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&texture_bind_group_layout],
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
                    rpass.set_bind_group(0, &diffuse_bind_group, &[]);
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
