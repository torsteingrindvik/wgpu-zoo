@group(0)
@binding(0)
var<uniform> u_time: f32;

@group(0)
@binding(1)
var t_read: texture_2d<f32>;

@group(0)
@binding(2)
var t_write: texture_storage_2d<r32float, write>;

@group(0)
@binding(3)
var s_sampler: sampler;

@group(0)
@binding(4)
var<uniform> u_mouse: vec2<u32>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var xy: vec2<f32>;

    let size = 0.9;

    if (input.vertex_index == 0u) {
        // bottom left
        xy = vec2<f32>(-1.0, -1.0) * size; 
    } else if (input.vertex_index == 1u) {
        // bottom right
        xy = vec2<f32>(1.0, -1.0) * size; 
    } else if (input.vertex_index == 2u) {
        // top left
        xy = vec2<f32>(-1.0, 1.0) * size; 
    } else {
        // top right
        xy = vec2<f32>(1.0, 1.0) * size; 
    }

    out.position = vec4<f32>(xy, 0., 1.);

    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coordinates = vec2<i32>(input.position.xy);

    let width_height = textureDimensions(t_read);

    let whf = vec2<f32>(width_height);
    let l_from_mouse = length(vec2<f32>((-vec2<i32>(u_mouse)) + pixel_coordinates) / whf);

    let draw_radius = 0.05;
    let dist01 = max(0., 1. - (l_from_mouse * (1. / draw_radius)));

    let posf = input.position.xy;

    // What's already stored
    var value = textureSample(t_read, s_sampler, (posf / whf)).r;

    // How much to add, based on distance from mouse
    let add = dist01 / 10.;

    // How much to remove per frame
    let fade = 0.001;

    let result = saturate(value + add - fade);

    let col = vec4<f32>(result, 0.1, 0.1, 1.);
    textureStore(t_write, pixel_coordinates, col);
    return col;
}
