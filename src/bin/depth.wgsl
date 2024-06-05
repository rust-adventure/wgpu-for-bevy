// struct VertexInput {
//     @location(0) position: vec3<f32>,
//     @location(1) color: vec3<f32>,
// };

// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
//     @location(0) color: vec3<f32>,
// }
// @vertex
// fn vertex(
//     model: VertexInput,
// ) -> VertexOutput {
//     var out: VertexOutput;
//     out.color = model.color;
//     out.clip_position = vec4<f32>(model.position, 1.0);
//     return out;
// }

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vertex(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(0) @binding(2)
var t_item_depth: texture_2d<f32>;
@group(0) @binding(3)
var s_item_depth: sampler;

// @fragment
// fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
//     return textureSample(t_diffuse, s_diffuse, in.tex_coords);
// }

struct FragmentOutput {
   @builtin(frag_depth) depth: f32,
   @location(0) color: vec4<f32>
 }

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    // let near = 0.1;
    // let far = 100.0;
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let depth = textureSample(t_item_depth, s_item_depth, in.tex_coords);
    // let r = (2.0 * near) / (far + near - depth * (far - near));
    // return vec4<f32>(vec3<f32>(r), 1.0);
    var out: FragmentOutput;
    out.depth = depth.x - 0.25;
    if depth.a < 0.1 {
        out.depth = 100.;
    }
    // out.color = vec4<f32>(1.,1.,1.,depth.w);
    out.color = color;
    return out;
    // return vec4<f32>(r.y,r.y,r.y, depth.w);
}