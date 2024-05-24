- use `WindowEvent::RedrawRequested` to render in response to

## Terms

- winit
  - EventLoop
- RenderPipeline
- Surface
- Device

## App

Winit is a low-level crate for handling window management. This includes creating and destroying windows as well as running an event loop that handles window resize, mouse, and key events.

We need to pair winit with another crate to actually render content inside of the window that was created. For us, that's going to be wgpu.

We'll be focusing on desktop platforms to avoid getting into edge cases for mobile and wasm, but generally speaking you can target whatever platform you want.

## winit's Event Loop

```rust
fn main() {
    tracing_subscriber::fmt().init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    event_loop.run_app(&mut app).expect("app to run")
}
```

Our `main` function is responsible for bootstrapping winit's [`EventLoop`](https://docs.rs/winit/0.30.0/winit/event_loop/struct.EventLoop.html) and setting up tracing, which we can use for logging messages to the console.

After creating a new `EventLoop` we can configure how our loop behaves by setting the [ControlFlow](https://docs.rs/winit/0.30.0/winit/event_loop/enum.ControlFlow.html#variant.Poll) using the `Poll` variant. The `ControlFlow::Poll` variant causes the event loop iteration to immediately begin a new loop when the current iteration finishes whether or not new events are available to process.

This can be good for applications like games and its what we'll use today, but you could also wait for events to come in instead.

We can use our `EventLoop` to drive our application using [`EventLoop::run_app`](https://docs.rs/winit/0.30.0/winit/event_loop/struct.EventLoop.html#method.run_app), which accepts an argument that implements the [`ApplicationHandler`](https://docs.rs/winit/0.30.0/winit/application/trait.ApplicationHandler.html) trait.

The struct we implement `ApplicationHandler` for will act as the state of our application (For us, this is `App`). The fields on this struct will be available whenever we need to process an event and its where we'll put the data we initialize when using wgpu.

We'll instantiate our `App` struct and pass an exclusive reference into `run_app`.

## App

We'll be working with `App` for the winit `ApplicationHandler` trait implementation, so here's what it looks like. The data `App` contains here is fairly arbitrary: there's nothing special about it other than the fields being some data that we want to initialize in `resumed` and use in `window_event`.

```rust
#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    config: Option<SurfaceConfiguration>,
    render_pipeline: Option<RenderPipeline>,
    surface: Option<Surface<'a>>,
    device: Option<Device>,
    queue: Option<Queue>,
}
```

## ApplicationHandler

The `ApplicationHandler` trait requires us to implement the [`resumed`](https://docs.rs/winit/0.30.0/winit/application/trait.ApplicationHandler.html#tymethod.resumed) and [`window_event`](https://docs.rs/winit/0.30.0/winit/application/trait.ApplicationHandler.html#tymethod.window_event) functions.

`resumed` gets called whenever
