# wgpu for bevy

A series of examples with the goal of building a foundational knowledge of wgpu which can then be used to understand how Bevy's renderer (built on top of wgpu) works.

## Examples

### triangle

![a triangle](./src/bin/triangle.avif)

The triangle example is a minimal example that integrates with winit to display a window and render a triangle.

Bevy uses winit in a much more complex way but this example should provide some basis for understanding the winit event handling in `bevy_winit`.

- [bevy_winit](https://github.com/bevyengine/bevy/tree/main/crates/bevy_winit)
