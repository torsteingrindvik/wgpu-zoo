@group(0)
@binding(0)
var<uniform> u_time: f32;

struct VertexInput {
    // Will draw in total 64 verts
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct FragOutput {
  @location(0) fb0: vec4<f32>,
}

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    if ((input.vertex_index % 2u) == 0u) {
        // Even, place starting vertex at origin
        out.position = vec4<f32>(0.0, 0.0, 0.0, 1.0); 
    } else {
        // Odd, indices 1, 3, 5, 7, .., 63.
        // Make the angle radians based on vertex index.
        let rads = 2. * 3.1415 * f32(input.vertex_index) / 64.;
        // Spin!
        let t = (u_time * .2) + rads;

        out.position = vec4<f32>(cos(t), sin(t), 0.0, 1.0); 
    }

    return out;
}

@fragment
fn fs(input: VertexOutput) -> FragOutput {
    var out: FragOutput;

    out.fb0 = vec4(1.);

    return out;
}
