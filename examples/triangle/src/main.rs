use futures_lite::future::block_on;
use std::{borrow::Cow, sync::Arc};
use tracing::info;
use wgpu::{
    Device, Queue, RenderPipeline, Surface,
    SurfaceConfiguration,
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
}

#[derive(Default)]
struct App<'a> {
    resumed_data: Option<ResumedData<'a>>,
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

                info!(adapter=?adapter.get_info());

                // Create the logical device and command
                // queue
                let (device, queue) = adapter
                    .request_device(
                        &wgpu::DeviceDescriptor::default(),
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

        // Load the shaders from disk
        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: "triangle_shader".into(),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(include_str!(
                        "triangle.wgsl"
                    )),
                ),
            },
        );

        let pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: "triangle_layout".into(),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                },
            );

        let swapchain_format =
            surface.get_capabilities(&adapter).formats[0];

        let render_pipeline = device
            .create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: "triangle_pipeline".into(),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vertex".into(),
                        buffers: &[],
                        compilation_options:
                            Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fragment".into(),
                        compilation_options:
                            Default::default(),
                        targets: &[Some(
                            swapchain_format.into(),
                        )],
                    }),
                    primitive:
                        wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample:
                        wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
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
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.draw(0..3, 0..1);
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
