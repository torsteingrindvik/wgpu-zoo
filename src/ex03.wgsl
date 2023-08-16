@binding(0)
var<uniform> u_instances: u32;

@group(0)
@binding(1)
var<uniform> u_radius: f32;

@group(0)
@binding(2)
var<uniform> u_mouse: vec2<f32>;

@group(0)
@binding(3)
var<uniform> u_time: f32;

struct VertexInput {
    @builtin(instance_index) ii: u32,
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ii: f32,
};

@vertex
fn vs(vertex: VertexInput) -> VertexOutput {
    var iif = f32(vertex.ii);
    var a = (2. * 3.1415) * (iif / f32(u_instances));
    var rot = mat2x2f(cos(a), -sin(a), sin(a), cos(a));
    var scale = mat2x2f(0.1, 0., 0., 0.1);

    var offset = (3.0 + sin(u_time*5. + a)) * vec2<f32>(u_radius, u_radius);

    var pos = rot * scale * (vertex.position + offset);

    var v: VertexOutput;

    v.position = vec4<f32>(pos.x + u_mouse.x, pos.y + u_mouse.y, 0.0, 1.0);
    v.ii = iif;

    return v;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.2, 0.3, (sin(u_time * 8. + (input.ii + 1.) * 10.) + 1.) / 2., 1.0);
}
