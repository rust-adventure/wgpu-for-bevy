use bytemuck::{Pod, Zeroable};
use image::{GenericImageView, ImageError};
use wgpu_for_bevy::{texture::Texture, DepthPass, Vertex};
use std::{borrow::Cow, sync::Arc};
use tracing::info;
use wgpu::{
    util::DeviceExt, BlendComponent, BlendState, ColorTargetState, Device, PrimitiveState, Queue, RenderPipeline, Surface, SurfaceConfiguration
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};


const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 1., 0.0],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -1., 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -1., 0.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 1., 0.0],
        tex_coords: [1.0, 0.0],
    },
];
const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];
// const VERTICES_PLAYER: &[Vertex] = &[
//     Vertex {
//         position: [0.75 + -0.25, 0.75 + 0.25, 0.0],
//         tex_coords: [0.0, 0.0],
//     },
//     Vertex {
//         position: [0.75 + -0.25, 0.75 + -0.25, 0.0],
//         tex_coords: [0.0, 1.0],
//     },
//     Vertex {
//         position: [0.75 + 0.25, 0.75 + -0.25, 0.0],
//         tex_coords: [1.0, 1.0],
//     },
//     Vertex {
//         position: [0.75 + 0.25, 0.75 + 0.25, 0.0],
//         tex_coords: [1.0, 0.0],
//     },
// ];
const INDICES_PLAYER: &[u16] = &[0, 1, 2, 0, 2, 3];

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    config: Option<SurfaceConfiguration>,
    render_pipeline: Option<RenderPipeline>,
    surface: Option<Surface<'a>>,
    device: Option<Device>,
    queue: Option<Queue>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    vertex_buffer_player: Option<wgpu::Buffer>,
    index_buffer_player: Option<wgpu::Buffer>,
    depth_texture: Option<Texture>,
    diffuse_bind_group: Option<wgpu::BindGroup>,
    diffuse_texture: Option<Texture>,
    diffuse_bind_group_player: Option<wgpu::BindGroup>,
    player_position: usize,
    depth_pass: Option<DepthPass>
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        let window_attributes =
            Window::default_attributes()
                .with_title("wgpu for Bevy");

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .unwrap(),
        );
        self.window = Some(window.clone());

        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();

        let (surface, adapter, device, queue) =
            futures_lite::future::block_on(async move {
                let surface = instance
                    .create_surface(window)
                    .unwrap();

                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference:
                            wgpu::PowerPreference::default(),
                        force_fallback_adapter: false,
                        // Request an adapter which can render to our surface
                        compatible_surface: Some(&surface),
                    })
                    .await
                    .expect(
                        "Failed to find an appropriate adapter",
                    );

                // Create the logical device and command
                // queue
                let (device, queue) = adapter
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            label: None,
                            required_features: wgpu::Features::empty(),
                            // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                            required_limits:
                                wgpu::Limits::downlevel_webgl2_defaults(
                                )
                                .using_resolution(adapter.limits()),
                        },
                        None,
                    )
                    .await
                    .expect("Failed to create device");

                (surface, adapter, device, queue)
            });

        let diffuse_bytes = include_bytes!("doorway_E.png");
        let diffuse_texture = Texture::from_bytes(
            &device,
            &queue,
            diffuse_bytes,
            "doorway_E.png",
        )
        .unwrap();

        let diffuse_bytes_depth = include_bytes!("doorway_E_depth.png");
        let diffuse_texture_depth = Texture::from_bytes(
            &device,
            &queue,
            diffuse_bytes_depth,
            "doorway_E_depth.png",
        )
        .unwrap();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_depth.view), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture_depth.sampler), 
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        self.diffuse_bind_group = Some(diffuse_bind_group);

        let diffuse_bytes = include_bytes!("player.png");
        let diffuse_texture = Texture::from_bytes(
            &device,
            &queue,
            diffuse_bytes,
            "player.png",
        )
        .unwrap();

        let diffuse_bytes_depth = include_bytes!("player.png");
        let diffuse_texture_depth = Texture::from_bytes(
            &device,
            &queue,
            diffuse_bytes_depth,
            "player.png",
        )
        .unwrap();
        // let texture_bind_group_layout_player =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Texture {
        //                     multisampled: false,
        //                     view_dimension: wgpu::TextureViewDimension::D2,
        //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                 },
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 // This should match the filterable field of the
        //                 // corresponding Texture entry above.
        //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                 count: None,
        //             },
        //         ],
        //         label: Some("texture_bind_group_layout_player"),
        //     });

        let diffuse_bind_group_player = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_depth.view), 
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture_depth.sampler), 
                    }
                ],
                label: Some("diffuse_bind_group_player"),
            }
        );

        self.diffuse_bind_group_player =
            Some(diffuse_bind_group_player);

        self.queue = Some(queue);

        // Load the shaders from disk
        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(include_str!(
                        "depth.wgsl"
                    )),
                ),
            },
        );

        let pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                },
            );

        let swapchain_capabilities =
            surface.get_capabilities(&adapter);
        let swapchain_format =
            swapchain_capabilities.formats[0];

            dbg!(&swapchain_format);
        let render_pipeline = device
            .create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vertex",
                    buffers: &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fragment",
                    compilation_options: Default::default(),
                    // targets: &[Some(
                    //     swapchain_format.into(),
                    // )],
                    targets: &[
                        Some(ColorTargetState{
                            format: swapchain_format,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })
                    ]
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(
                    wgpu::DepthStencilState {
                        format: Texture::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare:
                            wgpu::CompareFunction::Less,
                        stencil:
                            wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(
                        ),
                    },
                ),
                multisample:
                    wgpu::MultisampleState::default(),
                multiview: None,
            },
        );

        let config = surface
            .get_default_config(
                &adapter,
                size.width,
                size.height,
            )
            .unwrap();

        surface.configure(&device, &config);
        let depth_pass = DepthPass::new(&device, &config);
        self.depth_pass = Some(depth_pass);

        let depth_texture = Texture::create_depth_texture(
            &device,
            &config,
            "depth_texture",
        );
        self.depth_texture = Some(depth_texture);

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );
        self.vertex_buffer = Some(vertex_buffer);
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );
        self.index_buffer = Some(index_buffer);

        // let vertex_buffer_player = device
        //     .create_buffer_init(
        //         &wgpu::util::BufferInitDescriptor {
        //             label: Some("Vertex Buffer"),
        //             contents: bytemuck::cast_slice(
        //                 VERTICES_PLAYER,
        //             ),
        //             usage: wgpu::BufferUsages::VERTEX,
        //         },
        //     );
        // self.vertex_buffer_player =
        //     Some(vertex_buffer_player);
        let index_buffer_player = device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer Player"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                },
            );
        self.index_buffer_player =
            Some(index_buffer_player);

        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.surface = Some(surface);
        self.device = Some(device);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // info!("{event:?}");

        match event {
            WindowEvent::CloseRequested => {
                // There is a bug on macos which panics when
                // the window closes. This
                // isn't a huge deal since the application
                // is already closing, but theoretically
                // would prevent cleanup (since it is a
                // panic) and is ugly from a DX/UX
                // perspective.
                //
                // ```
                // a delegate was not configured on the application
                // ```
                //
                // that can be worked around by taking the
                // window and dropping it here ourselves.
                // the fix has already been merged, but is
                // not in a winit release yet. https://github.com/rust-windowing/winit/pull/3684
                //
                // we use `.take()` to replace the options
                // with `None` in our `App`, then we own the
                // data and it will drop.
                //
                // `surface` keeps a reference to the
                // window, so we need to drop that first
                let _ = self.surface.take();
                // then we can drop the window
                let _ = self.window.take();

                event_loop.exit();
            }
            WindowEvent::Resized(PhysicalSize {
                width,
                height,
            }) => {
                let Self {
                    config: Some(config),
                    surface: Some(surface),
                    device: Some(device),
                    window: Some(window),
                    depth_texture: Some(depth_texture),
                    ..
                } = self
                else {
                    return;
                };

                // Reconfigure the surface with the new size
                config.width = width.max(1);
                config.height = height.max(1);

                surface.configure(device, config);
                *depth_texture =
                    Texture::create_depth_texture(
                        &device,
                        &config,
                        "depth_texture",
                    );
                // On macos the window needs to be redrawn
                // manually after resizing
                window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // allow a single_match here so that people
                // who use this example can easily match on
                // new keys
                #[allow(clippy::single_match)]
                match key.as_ref() {
                    // WARNING: Consider using
                    // `key_without_modifiers()` if
                    // available on your platform.
                    Key::Named(NamedKey::Escape) => {
                        // TODO: This is the same handling
                        // as `WindowEvent::CloseRequested`,
                        // which we'll be removing in a
                        // future version
                        let _ = self.surface.take();
                        // then we can drop the window
                        let _ = self.window.take();

                        event_loop.exit();
                    }
                    _ => (),
                }
            }
            WindowEvent::RedrawRequested => {
                let Self {
                    surface: Some(surface),
                    device: Some(device),
                    queue: Some(queue),
                    render_pipeline: Some(render_pipeline),
                    ..
                } = self
                else {
                    return;
                };

                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame.texture.create_view(
                    &wgpu::TextureViewDescriptor::default(),
                );
                
                let mut encoder = device
                    .create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: None,
                        },
                    );
                {
                    let mut render_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color{
                                    r: 0.008,
                                    g: 0.024,
                                    b: 0.09,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            // view: &self.depth_texture.as_ref().unwrap().view,
                            view: &self.depth_pass.as_ref().unwrap().texture.view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass
                        .set_pipeline(render_pipeline);
                    render_pass.set_bind_group(
                        0,
                        &self
                            .diffuse_bind_group
                            .as_ref()
                            .unwrap(),
                        &[],
                    );

                    render_pass.set_vertex_buffer(
                        0,
                        self.vertex_buffer
                            .as_ref()
                            .unwrap()
                            .slice(..),
                    );
                    render_pass.set_index_buffer(
                        self.index_buffer
                            .as_ref()
                            .unwrap()
                            .slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    render_pass.draw_indexed(
                        0..INDICES.len() as u32,
                        0,
                        0..1,
                    );

                    self.player_position = (self.player_position + 1) % 100;
               
                   let offset_player_x = -(self.player_position as f32 / 100.);
                   let offset_player_y = -0.45;
                   let player_z = 0.5;
                    let vertices = [
                        Vertex {
                            position: [offset_player_x + 0.,offset_player_y +  0.25, player_z],
                            tex_coords: [0.0, 0.0],
                        },
                        Vertex {
                            position: [offset_player_x + 0.,offset_player_y +  -0.25, player_z],
                            tex_coords: [0.0, 1.0],
                        },
                        Vertex {
                            position: [offset_player_x + 0.5, offset_player_y + -0.25, player_z],
                            tex_coords: [1.0, 1.0],
                        },
                        Vertex {
                            position: [offset_player_x + 0.5, offset_player_y + 0.25, player_z],
                            tex_coords: [1.0, 0.0],
                        },
                    ];

                    self.vertex_buffer_player = Some(
                    device
                    .create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(
                                &vertices,
                            ),
                            usage: wgpu::BufferUsages::VERTEX,
                        },
                    ));
                    
                    // let vertex_buffer_player = device
                    // .create_buffer_init(
                    //     &wgpu::util::BufferInitDescriptor {
                    //         label: Some("Vertex Buffer"),
                    //         contents: bytemuck::cast_slice(
                    //             VERTICES_PLAYER,
                    //         ),
                    //         usage: wgpu::BufferUsages::VERTEX,
                    //     },
                    // );

                    render_pass.set_bind_group(
                        0,
                        &self
                            .diffuse_bind_group_player
                            .as_ref()
                            .unwrap(),
                        &[],
                    );
                    render_pass.set_vertex_buffer(
                        0,
                        self.vertex_buffer_player
                            .as_ref()
                            .unwrap()
                            .slice(..),
                    );
                    render_pass.set_index_buffer(
                        self.index_buffer_player
                            .as_ref()
                            .unwrap()
                            .slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    render_pass.draw_indexed(
                        0..INDICES_PLAYER.len() as u32,
                        0,
                        0..1,
                    );
                }

                self.depth_pass.as_mut().unwrap().render(&view, &mut encoder);
                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            _ => (),
        }
    }
    fn about_to_wait(
        &mut self,
        _event_loop: &ActiveEventLoop,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        window.request_redraw();
    }
}

fn main() {
    tracing_subscriber::fmt().init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    event_loop.run_app(&mut app).expect("app to run")
}
