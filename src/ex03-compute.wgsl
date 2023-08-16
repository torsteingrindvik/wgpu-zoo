struct Params {
    mouse: vec2<f32>,
};

@group(0)
@binding(0)
var<uniform> params: Params;

struct Circle {
    pos: f32,
};

@group(0)
@binding(1)
var<storage, read_write> circles : array<Circle>;

@compute
@workgroup_size(64)
fn cs(@builtin(global_invocation_id) giid: vec3<u32>) {
    // todo
}