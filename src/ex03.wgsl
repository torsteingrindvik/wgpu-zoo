@group(0)
@binding(0)
var<uniform> u_viewport: vec2<f32>;

@group(0)
@binding(1)
var<uniform> u_quad: array<vec4<f32>, 4>;

@group(0)
@binding(2)
var<uniform> u_mouse: vec2<f32>;

@group(0)
@binding(3)
var<uniform> u_time: f32;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs(vertex: VertexInput) -> VertexOutput {
    var v: VertexOutput;

    v.position = vec4<f32>(vertex.position.x , vertex.position.y, 0.0, 1.0);

    return v;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    // See: https://www.w3.org/TR/WGSL/#built-in-values-position
    // Since we're using a builtin, some things are applied to it.
    //
    // Quote:
    /*
        The framebuffer is a two-dimensional grid of pixels with the
        top-left at (0.0, 0.0) and the bottom right at (vp.width, vp.height).
        Each pixel has an extent of 1.0 unit in each of the x and y dimensions,
        and pixel centers are at (0.5,0.5) offset from integer coordinates.
    */
    // This means that in order to relate this to the mouse uniform,
    // we have to know the size of the viewport, which we have now added as u_viewport.

    // Normalize again to 0.0->1.0;
    var pos = input.position.xy / u_viewport;
    // Now to -1.0..1.0
    pos = (pos * 2.0) - 1.0;
    // Framebuffer coords have flipped y compared to clip
    pos.y *= -1.0;

    // We only care about the distance to the closest vertex- i.e. the one with the least distance
    var closest = 10.;
    var vertex = u_quad[0].xy;

    for (var i: i32 = 0; i < 4; i++) {
        // How far to quad vertex
        let l = length(-u_mouse + u_quad[i].xy);
        if (l < closest) {
            closest = l;
            vertex = u_quad[i].xy;
        }
    }

    let v_pos = -vertex + pos;
    let v_mouse = -vertex + u_mouse;

    let lv_pos = length(v_pos);
    let lv_mouse = length(v_mouse);

    // cos of pos, mouse
    let ang = dot(v_pos, v_mouse) / lv_pos / lv_mouse;

    var color = vec4<f32>(0.8, 0.0, 0.0, 1.0);
    if (lv_pos < lv_mouse && lv_mouse < 0.5) {
        var signal = lv_pos;
        // If 
        signal -= 0.2 * ang;
        
        var dist = 1. - signal;

        // Re-shape: Make it taper off a lot harder.
        dist = pow(dist, 25.);

        color.y = dist;
    }

    return color;
}
