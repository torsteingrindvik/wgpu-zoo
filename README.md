# wgpu zoo

Just visiting the different animals I can find within wgpu.
The goal is just to poke at things to see what I can learn.

## TODOs

- ~~Make `PolygonMode` hotkey available to change for all examples? If requested of an example the pipeline could just be recreated to use the new one.~~
    - Done via keys Up/Down/W/S and marking dirty
- ~~X button to close window~~
    - Done via `WindowEvent::CloseRequested`
- Debug markers in vulkan?
- ~~Hot reload~~
    - Done via `notify` on `wgsl` file changes
- Catch bad compile of wgsl?
- Allow spoofing mouse movements
- Handle resize (recreate textures, mark dirty)
- Add description to example trait, such that when we P/N to switch we can println what's going on

## Ideas

- Scissor rect around cursor, two passes, one shows fill polygon mode, other shows something else. Would make something like an x-ray effect?
    If it could be combined with gltf etc. that would be neat

## Example harness

Examples implement a trait. When examples run via `winit`, key events are passed down, as well as delta time.
The trait has a core function `render`.

## Example 1: Red triangle

A red triangle via three vertices in a vertex buffer.

### Controls

Arrow keys to move the triangle around.

## Example 2: Instanced triangles, polygon modes

A ring of instanced wavy triangles with some color changing.

Also sees the effect of polygon modes on geometry.

### Controls

W/S for changing polygon mode.
A/D for changing ring radius.

Scroll wheel to change number of instances.

Move mouse to have the ring follow.

## Example 3: Moving quad

* A quad via four vertices and a triangle strip (instead of the normal list)
* If mouse is close to a vertex, area close to vertex turns green to indicate "selectable"
* Mouse to click and hold to move vertex
* Push constant to adjust proximity threshold (via scroll)

### Controls

Mouse to hover, then press to select, hold down and move then release to place.
Scroll wheel to increase/decrease threshold.

## Example 4: Several render attachments

See [here](https://gpuweb.github.io/gpuweb/wgsl/#example-ee897116).

The general syntax is:

```wgsl
struct MyOutputs {
  @location(0) foo: vec4<f32>
  @location(1) bar: vec4<f32>
  @location(2) qux: vec4<f32>
}
```

Which is interesting just to try.
The example makes a triangle (slightly skewered) and renders it in a single pass to two render attachments.
One is the screen, the other is offscreen.

The offscreen one isn't saved, so it's only viewable through something like renderdoc.

## Example n: Compute into render

The goal is to use a compute shader to generate geometry, then render that.

So the plan is:

* Have the compute shader generate circles
* Let's generate something like 100x100 circles
* Let's make the circle color be based on the position on screen
* Is it possible to make the radius of the circles somehow a function of the distance from the cursor?
