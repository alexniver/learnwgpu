struct VertexInput{
    @location(0) pos: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct FragInput {
    @location(0) tex_coord:vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> FragInput {
    var fragInput : FragInput;
    fragInput.clip_position = vec4<f32>(input.pos, 1.0);
    fragInput.tex_coord = input.tex_coord;
    return fragInput;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, input.tex_coord);
}
