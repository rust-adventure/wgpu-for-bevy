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

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    config: Option<SurfaceConfiguration>,
    render_pipeline: Option<RenderPipeline>,
    surface: Option<Surface<'a>>,
    device: Option<Device>,
    queue: Option<Queue>,
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

                // Create the logical device and command queue
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

        self.queue = Some(queue);

        // Load the shaders from disk
        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(include_str!(
                        "shader.wgsl"
                    )),
                ),
            },
        );

        let pipeline_layout = device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                },
            );

        let swapchain_capabilities =
            surface.get_capabilities(&adapter);
        let swapchain_format =
            swapchain_capabilities.formats[0];

        let render_pipeline = device
            .create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[],
                        compilation_options:
                            Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
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
        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.surface = Some(surface);
        self.device = Some(device);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        info!("{event:?}");

        match event {
            WindowEvent::CloseRequested => {
                // There is a bug on macos which panics when the window closes. This
                // isn't a huge deal since the application is already closing, but theoretically
                // would prevent cleanup (since it is a panic) and is ugly from a DX/UX perspective.
                //
                // ```
                // a delegate was not configured on the application
                // ```
                //
                // that can be worked around by taking the window and dropping it here ourselves.
                // the fix has already been merged, but is not in a winit release yet.
                // https://github.com/rust-windowing/winit/pull/3684
                //
                // we use `.take()` to replace the options with `None` in our `App`, then we own the
                // data and it will drop.
                //
                // `surface` keeps a reference to the window, so we need to drop that first
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
                    ..
                } = self
                else {
                    return;
                };

                // Reconfigure the surface with the new size
                config.width = width.max(1);
                config.height = height.max(1);

                surface.configure(device, config);
                // On macos the window needs to be redrawn manually after resizing
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
                // allow a single_match here so that people who use this example can easily match on
                // new keys
                #[allow(clippy::single_match)]
                match key.as_ref() {
                    // WARNING: Consider using `key_without_modifiers()` if available on your platform.
                    Key::Named(NamedKey::Escape) => {
                        // TODO: This is the same handling as `WindowEvent::CloseRequested`,
                        // which we'll be removing in a future version
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
                    let mut rpass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rpass.set_pipeline(render_pipeline);
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
