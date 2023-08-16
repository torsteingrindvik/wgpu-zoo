struct VertexInput {
    @location(0) position: vec2<f32>,
}

@vertex
fn vs(vertex: VertexInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(vertex.position.x, vertex.position.y, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
