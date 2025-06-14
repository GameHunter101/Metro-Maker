struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @builtin(vertex_index) index: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4f(input.position, 1.0);
    out.tex_coords = input.tex_coords;
    return out;
}
