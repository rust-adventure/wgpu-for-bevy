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

### about_to_wait

```rust
fn about_to_wait(
    &mut self,
    _event_loop: &ActiveEventLoop,
) {
    let Some(window) = self.window.as_ref() else {
        return;
    };

    window.request_redraw();
}
```

We also implement `about_to_wait`, although this isn't strictly required. Our use case for `about_to_wait` is constantly requesting the window to redraw. This is one of those "its a simple example so we'll just always request a redraw" kinds of things.

The overall flow is

1. `EventLoop` finishes an iteration
2. The `ControlFlow` setting defines what the desired behavior is after the `Event::AboutToWait` event occurs.
3. The `Event::AboutToWait` event causes `about_to_wait` to be called.
4. We `request_redraw`, which creates a `WindowEvent::RedrawRequested` event for the next loop iteration to process.
5. _because_ we've requested a redraw, the `window_event` handler above will fire, and we will get the opportunity to draw to the screen.

There are [platform-specific reasons](https://docs.rs/winit/0.30.0/winit/window/struct.Window.html#method.request_redraw) a redraw can be requested. The winit docs talk about some of this inconsistency:

> There are no strong guarantees about when exactly a RedrawRequest event will be emitted with respect to other events, since the requirements can vary significantly between windowing systems.

More sophisticated applications like Bevy use [`RequestRedraw` events](https://github.com/bevyengine/bevy/blob/44c0325ecd5e8379be51426309eab47c12f6b289/crates/bevy_winit/src/lib.rs#L376-L380).
