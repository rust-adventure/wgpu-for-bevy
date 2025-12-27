use encase::{ShaderType, UniformBuffer};
use futures_lite::future::block_on;
use std::{sync::Arc, time::Instant};
use tracing::info;
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType,
    Device, ExperimentalFeatures, Limits, Queue,
    RenderPipeline, ShaderStages, Surface,
    SurfaceConfiguration, TaskState, util::DeviceExt,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

struct ResumedData<'a> {
    window: Arc<Window>,
    config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    time_bind_group: BindGroup,
    time_uniform_buffer: wgpu::Buffer,
}

struct App<'a> {
    resumed_data: Option<ResumedData<'a>>,
    start: Instant,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            resumed_data: Default::default(),
            start: Instant::now(),
        }
    }
}

#[derive(ShaderType)]
struct ShaderData {
    time: f32,
}

/// Winit
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

        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();

        // `surface_window` is captured by the async closure,
        // so we clone our Arc and let the closure take it
        let surface_window = window.clone();
        // wgpu apis for getting an adapter are async,
        // so we block while waiting for them to complete
        let (surface, adapter, device, queue) = block_on(
            async move {
                let surface = instance
                    .create_surface(surface_window)
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

                // info!(features=?adapter.features());
                // features we need
                let has_features = adapter.features().contains(
                    wgpu::Features::EXPERIMENTAL_MESH_SHADER | wgpu::Features::EXPERIMENTAL_PASSTHROUGH_SHADERS
                );
                if !has_features {
                    panic!(
                        "necessary features unavailable"
                    );
                };

                info!(adapter=?adapter.get_info());

                // Create the logical device and command
                // queue
                let (device, queue) = adapter
                    .request_device(
                        &wgpu::DeviceDescriptor{
                            label: Some("mesh_adapter"),
                            required_features:  wgpu::Features::EXPERIMENTAL_MESH_SHADER,
                            experimental_features: unsafe { ExperimentalFeatures::enabled() },
                            required_limits: Limits::default().using_recommended_minimum_mesh_shader_values(),
                           ..Default::default()
                        },
                    )
                    .await
                    .expect("Failed to create device");

                // device.limits will print the hard limits of the
                // device. This includes things like max texture dimensions,
                // max color attachments, and max vertex buffers.
                // info!(limits=?device.limits());
                (surface, adapter, device, queue)
            },
        );

        info!("build task_shader");
        let task_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("task_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("task.wgsl").into(),
                ),
            },
        );
        // metal requires passthrough... leaving this here for a moment until either wgsl -> metal merges or
        // I get around to writing separate metal shaders
        //                 let task_shader = unsafe { device.create_shader_module_passthrough(CreateShaderModuleDescriptorPassthrough{
        //                     entry_point: "task".into(),
        //                     label: Some("task_shader"),
        // num_workgroups: (1, 1, 1),
        //                     wgsl: Some(include_str!("task.wgsl").into()),
        //                     ..Default::default()
        //                 }

        //         ) };
        info!("build mesh_shader");
        let mesh_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("mesh_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("mesh.wgsl").into(),
                ),
            },
        );
        info!("build fragment_shader");
        let fragment_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("fragment_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("fragment.wgsl").into(),
                ),
            },
        );
        let time_layout = device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                label: "time_layout".into(),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            ShaderData::min_size(),
                        ),
                    },
                    count: None,
                }],
            },
        );
        let mut buffer =
            UniformBuffer::new(Vec::<u8>::new());
        let data = ShaderData {
            time: self.start.elapsed().as_secs_f32(),
        };
        buffer.write(&data).unwrap();
        let byte_buffer = buffer.into_inner();

        let time_uniform_buf = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("shader_data_uniform_buffer"),
                contents: &byte_buffer,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
            },
        );
        // uniform.wri
        let time_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: "time_bind_group".into(),
                layout: &time_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: time_uniform_buf
                        .as_entire_binding(),
                }],
            },
        );
        let pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: "triangle_layout".into(),
                    bind_group_layouts: &[&time_layout],
                    immediate_size: 0,
                },
            );

        let swapchain_format =
            surface.get_capabilities(&adapter).formats[0];

        let render_pipeline = device.create_mesh_pipeline(
            &wgpu::MeshPipelineDescriptor {
                label: "mesh_shader_pipeline".into(),
                layout: Some(&pipeline_layout),
                task: Some(TaskState {
                    module: &task_shader,
                    entry_point: "task".into(),
                    compilation_options: Default::default(),
                }),
                mesh: wgpu::MeshState {
                    module: &mesh_shader,
                    entry_point: "mesh".into(),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: "fragment".into(),
                    compilation_options: Default::default(),
                    targets: &[Some(
                        swapchain_format.into(),
                    )],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                cache: None,
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
        self.resumed_data = Some(ResumedData {
            window,
            config,
            render_pipeline,
            surface,
            device,
            queue,
            time_bind_group,
            time_uniform_buffer: time_uniform_buf,
        });
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
                event_loop.exit();
            }
            WindowEvent::Resized(PhysicalSize {
                width,
                height,
            }) => {
                let Some(ResumedData {
                    config,
                    surface,
                    device,
                    ..
                }) = self.resumed_data.as_mut()
                else {
                    return;
                };

                // Reconfigure the surface with the new size,
                // making it so that the window is *at least* 1x1
                config.width = width.max(1);
                config.height = height.max(1);

                surface.configure(&device, &config);
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
                        event_loop.exit();
                    }
                    _ => (),
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(ResumedData {
                    surface,
                    device,
                    queue,
                    render_pipeline,
                    ..
                }) = self.resumed_data.as_ref()
                else {
                    return;
                };

                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame.texture.create_view(
                    &wgpu::TextureViewDescriptor::default(),
                );

                let mut buffer =
                    UniformBuffer::new(Vec::<u8>::new());
                let data = ShaderData {
                    time: self
                        .start
                        .elapsed()
                        .as_secs_f32(),
                };
                dbg!(self.start.elapsed().as_secs_f32());

                buffer.write(&data).unwrap();
                let byte_buffer = buffer.into_inner();
                queue.write_buffer(
                    &self
                        .resumed_data
                        .as_ref()
                        .unwrap()
                        .time_uniform_buffer,
                    0,
                    &byte_buffer,
                );
                let mut encoder = device
                    .create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label:
                                "triangle_command_encoder"
                                    .into(),
                        },
                    );
                {
                    let mut rpass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: "triangle_render_pass".into(),
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
                            // depth_slice allows rendering to a layer of a texture array
                            // or a slice of a 3d texture view
                            depth_slice: None
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None
                    });
                    rpass.push_debug_group(
                        "Prepare data for draw.",
                    );
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(
                        0,
                        Some(
                            &self
                                .resumed_data
                                .as_ref()
                                .unwrap()
                                .time_bind_group,
                        ),
                        &[],
                    );
                    rpass.pop_debug_group();
                    rpass.insert_debug_marker("Draw!");
                    rpass.draw_mesh_tasks(1, 1, 1);
                }

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
        let Some(data) = self.resumed_data.as_ref() else {
            return;
        };

        data.window.request_redraw();
    }
}

fn main() {
    tracing_subscriber::fmt().init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    event_loop.run_app(&mut app).expect("app to run")
}
