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
* Color via vertex attr
* If mouse is close to a vertex, vertex turns green
* No culling such that we can invert

### Controls

Mouse to hover, then press to select, hold down and move then release to place

## Example n: Compute into render

The goal is to use a compute shader to generate geometry, then render that.

So the plan is:

* Have the compute shader generate circles
* Let's generate something like 100x100 circles
* Let's make the circle color be based on the position on screen
* Is it possible to make the radius of the circles somehow a function of the distance from the cursor?