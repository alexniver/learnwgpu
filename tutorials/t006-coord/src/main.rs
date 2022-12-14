use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use tracing::{info, Level};
use wgpu::{include_wgsl, Backends, Instance};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use wgpu::util::DeviceExt;

fn main() {
    tracing_subscriber::fmt().with_max_level(Level::WARN).init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    pollster::block_on(run(event_loop, window));
}

const TRANSLATE_SPEED: f32 = 1.;
const ROTATE_SPEED: f32 = 10.;
const SCALE_SPEED: f32 = 1.;

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
        vertex([-0.5, -0.5, 0.], [0., 1.]), // left bottom front
        vertex([0.5, -0.5, 0.], [1., 1.]),  // right bottom front
        vertex([0.5, 0.5, 0.], [1., 0.]),   // top right front
        vertex([-0.5, 0.5, 0.], [0., 0.]),  // top left front
    ];

    let indices = vec![
        0, 1, 3, // first triangle
        1, 2, 3, // second triangle
    ];

    (vertices, indices)
}

struct Transform {
    translation: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

impl Transform {
    fn new() -> Transform {
        Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            // scale: Vec3::NEG_ONE,
            // scale: Vec3::splat(0.5),
        }
    }

    fn rotate(&self, axis: Vec3, radius: f32) -> Transform {
        Transform {
            rotation: self.rotation * Quat::from_axis_angle(axis, radius),
            ..*self
        }
        // self.rotation = self.rotation + Quat::from_axis_angle(axis, radius);
        // self
    }

    fn rotate_x(&self, radius: f32) -> Transform {
        self.rotate(Vec3::X, radius)
    }

    fn rotate_y(&self, radius: f32) -> Transform {
        self.rotate(Vec3::Y, radius)
    }

    fn rotate_z(&self, radius: f32) -> Transform {
        self.rotate(Vec3::Z, radius)
    }

    pub(crate) fn set_scale(&self, scale: f32) -> Transform {
        Transform {
            scale: Vec3::splat(scale),
            ..*self
        }
    }

    pub(crate) fn add_translate(&self, tran_val: f32) -> Transform {
        Transform {
            translation: Vec3::new(
                self.translation.x + tran_val,
                self.translation.y + tran_val,
                self.translation.z,
            ),
            ..*self
        }
    }

    fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Mat4>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: mem::size_of::<[f32; 4 * 0]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: mem::size_of::<[f32; 4 * 1]>() as wgpu::BufferAddress,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: mem::size_of::<[f32; 4 * 2]>() as wgpu::BufferAddress,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: mem::size_of::<[f32; 4 * 3]>() as wgpu::BufferAddress,
                    shader_location: 5,
                },
            ],
        }
    }
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

    let diffuse_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
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

    // coord
    // let view = Mat4::look_at_rh(Vec3::new(0., 0., 3.), Vec3::ZERO, Vec3::Y);
    let view = Mat4::look_at_rh(Vec3::new(0., 0., 3.), Vec3::new(0., 1., 0.), Vec3::Y);
    let projection = Mat4::perspective_rh(
        // std::f32::consts::PI / 4.,
        (45.0 as f32).to_radians(),
        size.width as f32 / size.height as f32,
        0.1,
        40.,
    );

    // mat4X4 bindgroup layout
    let mat4_bindgroup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mat4x4 bindgroup layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(64),
            },
            count: None,
        }],
    });

    let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("View Buffer"),
        contents: bytemuck::cast_slice(view.as_ref()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let projection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Projection Buffer"),
        contents: bytemuck::cast_slice(projection.as_ref()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let view_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("view bind group"),
        layout: &mat4_bindgroup_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: view_buffer.as_entire_binding(),
        }],
    });

    let projection_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Projection Bindgroup Buffer"),
        layout: &mat4_bindgroup_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: projection_buffer.as_entire_binding(),
        }],
    });

    // shader
    let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            &texture_bind_group_layout, // group 0, texture
            &mat4_bindgroup_layout,     // group 1, view
            &mat4_bindgroup_layout,     // group 2, projection
        ],
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
            buffers: &[vertex_buffer_layout, Transform::buffer_layout()],
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

    // transform
    let now = Instant::now();
    let mut transform = Transform::new();

    let mut last_frame_game_time: f32 = 0.;

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &shader, &pipeline_layout);
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                let game_time = now.elapsed().as_secs_f32();
                let delta_time = game_time - last_frame_game_time;
                last_frame_game_time = game_time;

                info!("------------game time : {:?}", game_time);

                transform =
                    // transform.rotate_z((std::f32::consts::PI * delta_time).sin() * ROTATE_SPEED);
                    // transform.rotate_z(delta_time);
                transform.rotate_x(delta_time);

                transform = transform.add_translate(game_time.cos() / 100.);
                transform = transform.set_scale(game_time.sin().max(0.1));
                let mat4 = transform.to_mat4();
                let mut transform_buf =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Transform Buffer"),
                        contents: bytemuck::cast_slice(mat4.as_ref()),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

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
                        label: Some("Render Pass"),
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
                    rpass.set_bind_group(0, &diffuse_bindgroup, &[]);
                    rpass.set_bind_group(1, &view_bindgroup, &[]);
                    rpass.set_bind_group(2, &projection_bindgroup, &[]);
                    rpass.set_vertex_buffer(0, vertices_buf.slice(..)); // vertex_buffer
                    rpass.set_vertex_buffer(1, transform_buf.slice(..)); // transform mat4 buffer
                    rpass.set_index_buffer(indices_buf.slice(..), wgpu::IndexFormat::Uint16);

                    // rpass.draw(0..3, 0..1);
                    rpass.draw_indexed(0..indices.len() as u32, 0, 0..1)
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::RedrawEventsCleared => {
                info!("----------------------------------- redraw ");
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
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
