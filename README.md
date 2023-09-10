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

## Example 5: Scissor rect, MSAA

Uses scissor rect to draw a whole screen across two passes.
Draws left side then right side.
The right side load the results of the left then continues.

Shows a spinning circle of lines. One side has MSAA enabled, the other does not.

## Example 6: Set viewport

Tracks the mouse cursor, splitting example 05 into four quadrants.

## Example 7: Storage texture mouse drawing

- Draw to a storage texture: Color is added proportional to distance to mouse cursor
- Storage texture is write only, so use another texture to read (sample) from
- The previous frame's storage is the next frame's sampled texture

## Example 8: Texture array v1

- Create lots of textures
    - From the limits printout:
    > max_texture_array_layers: 256
    ~~so let's do that~~
    ended up via `[Texture; 256]`, try layers in example 9 instead
    - So we go to try `binding_array<texture_2d<f32>>` 
- Line them up on many quads
- Make them slightly transparent
- Make them slightly different colors
- ~~Allow "exploding" the textures outwards to separate them~~
- We also ended up trying using a single uniform buffer to store data for _all_ instances,
    indexing into it via the instance index.
    - So we used `array<mat3x3<f32>, 256>`

## Example 9: Texture array v2
Now we'll try the same as above, but with some changes:

- Use a single texture with 256 layers
- Use per-instance data via a second vertex buffer
- Upgrade to `draw_indexed`
- Potentially some mouse movement stuff?

## Example n: Draw with cursor (frag)

- Mouse is passed via uniform
- For each pixel, add some amount when mouse clicked
- Amount added proportional to length from mouse (and delta time)
- Two textures, one for writing one for reading?
    - https://www.w3.org/TR/WGSL/#texturestore
    Seems we can only write to a storage texture.
    So then we have to write to that, then get the results into the framebuffer after?

## Example n: Draw with cursor (compute)

Same as above but compute shader based.

- Only run compute pass when mouse is held down

## Example n: Compute into render

The goal is to use a compute shader to generate geometry, then render that.

So the plan is:

* Have the compute shader generate circles
* Let's generate something like 100x100 circles
* Let's make the circle color be based on the position on screen
* Is it possible to make the radius of the circles somehow a function of the distance from the cursor?

## Example n: LOD visualization

* Have some sort of geometry in a 3D scene
* Shade the color of the geometry based on the _screen size_
  * If done correctly, this means we could in theory sample smaller/less detailed textures based on the screen size

## Example n: Compute pass full screen

Info here: https://developer.nvidia.com/blog/advanced-api-performance-shaders/


> A good starting point is to target a thread group size corresponding to between two or eight warps. For instance, thread group size 8x8x1 or 16x16x1 for full-screen passes. Make sure to profile your shader and tune the dimensions based on profiling results.

Not sure exactly how this would play out, but it would be interesting to do some profiling of a full-screen compute shader which tried variations of those sizes, but also other more "wrong" sizes.
