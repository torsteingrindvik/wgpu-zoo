struct TimeMouse {
    time: f32,
    mouse: vec2<f32>,
};

@group(0)
@binding(0)
var<uniform> u_time_mouse: TimeMouse;

@group(0)
@binding(1)
var<uniform> u_affine: array<mat3x3<f32>, 256>;

@group(0)
@binding(2)
var ts: binding_array<texture_2d<f32>>;

@group(0)
@binding(3)
var s: sampler;

struct VertexInput {
    @builtin(instance_index) ii: u32,
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ii: u32,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.ii = input.ii;

    let pos = u_affine[input.ii] * vec3<f32>(input.position, 1.);
    out.position = vec4<f32>(pos.xy, 0., 1.);

    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(
        ts[input.ii],
        s,
        // Just sample the middle of the texture
        vec2<f32>(0.5, 0.5),
    );
}
