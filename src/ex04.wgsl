struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

struct FragOutput {
  @location(0) fb0: vec4<f32>,
  @location(1) fb1: vec4<f32>
}

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Expect draw 3.
    // Yes- there is a trick where this is done branchless, but it doesn't matter to this small example.

    if(input.vertex_index == 0u) {
        // Note: The *0.5 will not have any effect, since the last entry of the vec4 will also then be *0.5,
        // which is automatically adjusted for.
        out.position = vec4<f32>(-1.0, -1.0, 0.0, 1.0)*0.5; 
    } else if (input.vertex_index == 1u) {
        // This leaves the `w` part alone, which then _does_ mean this vertex will be moved by *0.9.
        out.position = vec4<f32>(1.0*0.9, -1.0*0.9, 0.0, 1.0); 
    } else {
        out.position = vec4<f32>(0.0, 1.0, 0.0, 1.0); 
    }

    return out;
}

@fragment
fn fs(input: VertexOutput) -> FragOutput {
    var out: FragOutput;

    out.fb0 = vec4(1., 0., 0., 1.);
    // Will turn out green anyway since we selectively enable only green & blue channels.
    out.fb1 = vec4(1., 1., 0., 1.);

    return out;
}
