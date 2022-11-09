struct VertexInput{
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct FragInput {
    @location(0) color:vec4<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> FragInput {
    var fragInput : FragInput;
    fragInput.clip_position = vec4<f32>(input.pos, 1.0);
    fragInput.color = input.color;
    return fragInput;
}

@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    return input.color;
}
