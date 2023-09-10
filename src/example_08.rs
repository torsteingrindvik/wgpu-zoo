use std::{mem::size_of, num::NonZeroU32};

use bytemuck::{Pod, Zeroable};

/*
Goals:
- Create lots of textures
    - From the limits printout:
    > max_texture_array_layers: 256
    so let's do that
    UPDATE: We ended up with layers=1 and 256 separate 1px textures instead,
    let's do the layers in the next example.

- Line them up on many quads
- Make them slightly transparent
- Make them slightly different colors
- Allow "exploding" the textures outwards to separate them

Learned:
- Even though we are drawing instanced,
    we still might want to have vertex stepped vertex buffers.
    This type will "reset" for each instance (I think, TODO),
    while an instanced stepped will +1 idx for each instance?

    Looking at wgpu's boids example it looks like that is the case.
    If something is instance-stepped, a vertex buffer bound as such must
    have enough data for each instance.
    So 100 instances and if the attrs on that is e.g. Float32x4 Float32x4 Float32x2,
    then you'd need 100 * (4*4 + 4*4 + 4*2) bytes total.

    I just found this: https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html#method.draw
    which has this pseudocode:

    ```rust
    for instance_id in instance_range {
        for vertex_id in vertex_range {
            let vertex = vertex[vertex_id];
            vertex_shader(vertex, vertex_id, instance_id);
        }
    }
    ```

    which explains things much more succinctly, indeed the vertex index (`vertex_id`) is just an inner loop.
    Great!

- We can make one big uniform buffer with data for each instance by simply indexing into it via the instance index.
    If we declare it to be array<mat3x3<f32>> in the shader and index into it via the instance index, the 0th entry is at byte offset 0,
    and the second is at byte offset 48, which is 12 f32s down the line!
    This can be surprising since mat3x3<f32> only contains 9 f32s.

    Some info was found here: https://www.w3.org/TR/WGSL/#alignment-and-size
    It states that SizeOf(array<vecR, C>) for mat3x3<f32> is 48, and its alignment is 16.
    We also read about Stride:

    > ...equals the size of the array’s element type, rounded up to the alignment of the element type

    So our size is 48, and that aligns with 16 already so no need to round up further.

- Writing column major order data is surprising.
  If we simply write [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] (but f32s), in RenderDoc we observe the mat3x3 to then be

  row1: [1, 5, 9]
  row2: [2, 6, 10]
  row3: [3, 7, 11]

  Which means we have to write padding interleaved, instead of the padding all ending up
  at the end!

- I'm not quite sure what the difference is between doing it the way I've done it here with a uniform buffer for the affine matrices versus
    just having instance data.
- Kinda the same note, what difference is there in having [Texture; 256] vs. Texture with 256 layers.

- If we want to first scale then translate in 2D, we use a matrix of the form:
    sx  0   tx
    0   sy  ty
    0   0   1

  and multiply that with the vertices which we define via [x, y] (two components).
  This means we have to extend the vertex position [x, y] to [x, y, 1], and I initially
  extended it in the shader as vec3<f32>(pos, 0.), which won't work.

   [sx  0   tx] [x]   [sx*x + tx*a]
   [0   sy  ty] [y] = [sy*y + ty*a]
   [0   0   1 ] [a]   [a]

  so notice then that if we forget to set a=1 but do a=0 instead, we get

    [sx*x]
    [sy*y]
    [0]

  instead of (a=1)

    [sx*x + tx]
    [sy*y + ty]
    [1]

  so interestingly the scaling still works (and it did, all quads were small and centered around the origin),
  but the translation is gone.
 */
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferUsages, Color, CommandEncoderDescriptor, Extent3d,
    FragmentState, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor, VertexBufferLayout, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example08 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    bgl0: BindGroupLayout,
    sampler: Sampler,
    textures: [Texture; 256],
    quad: Buffer,
}

impl Example for Example08 {
    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }

    fn handle_key(&mut self, key: winit::event::VirtualKeyCode) {
        match key {
            // Recreates pipeline and clears textures.
            winit::event::VirtualKeyCode::Space => self.common.dirty = true,
            _ => {}
        }
    }
}

impl Example08 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex08.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex08");

        println!("Creating textures with format {texture_format:?}");
        let textures: [Texture; 256] = (0..16)
            .into_iter()
            .flat_map(|col| {
                (0..16).into_iter().map(move |row| {
                    e.device.create_texture_with_data(
                        &e.queue,
                        &TextureDescriptor {
                            label: Some(&format!("ex08-texture-{col}-{row}")),
                            size: Extent3d {
                                width: 1,
                                height: 1,
                                // TODO: Make swappable
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::Rgba8Unorm,
                            usage: TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                        &[
                            ((col as f32 / 16.) * 256.0) as u8,
                            ((row as f32 / 16.) * 256.0) as u8,
                            0,
                            100,
                        ],
                    )
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex08-bgl0".into(),
                entries: &[
                    // Time, mouse
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Affine 3x3 f32 matrices
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: Some(NonZeroU32::new(textures.len() as u32).unwrap()),
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sampler = e.device.create_sampler(&SamplerDescriptor::default());

        let quad: wgpu::Buffer = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex08-buf".into(),
            contents: bytemuck::cast_slice(&[
                [-1.0f32, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
                [1.0, 1.0],
                [-1.0, -1.0],
                [1.0, -1.0],
            ]),
            usage: BufferUsages::VERTEX,
        });

        Self {
            render_pipeline: None,
            common,
            bgl0,
            textures,
            sampler,
            quad,
        }
    }

    fn render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        let texture_format = e.swapchain_format;

        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex08-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex08-rpass-pld".into(),
                bind_group_layouts: &[&self.bgl0],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &self.common.shader_module,
                entry_point: "vs",
                buffers: &[VertexBufferLayout {
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    // How many bytes in a _single_ thing.
                    // Thing is decided by `step_mode`, so how many
                    // bytes is used per vertex?
                    // In other words, each time we increase the thing (vertex) idx,
                    // how many bytes should we jump
                    array_stride: (size_of::<f32>() * 2) as _,
                    step_mode: wgpu::VertexStepMode::Vertex,
                }],
            },
            fragment: Some(FragmentState {
                module: &self.common.shader_module,
                entry_point: "fs",
                targets: &[Some(texture_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: self.common.polygon_mode,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn do_render(&mut self, e: &ExampleData) {
        // Command encoder begin
        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex08-ce".into(),
        });

        if self.common.dirty || self.render_pipeline.is_none() {
            self.common.dirty = false;
            self.render_pipeline = Some(self.render_pipeline(&e));
        }

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let screen_view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        #[repr(C)]
        #[derive(Clone, Copy, Pod, Zeroable)]
        struct TimeMouse {
            _time: f32,
            _mouse: [u32; 2],
        }

        // 256 quads, 16 per row, 16 per column.
        // Scale them down to fit one in each slot, and scale them further down to have a little border.
        let size: f32 = (1. / 16.) * 0.9;

        // Translation starts in the bottom left corner
        // let start = Vec2::new(-1., -1.);
        // Each offset should add this much in order for the last ones to be up at (+1., +1.)
        // let offset = Vec2::new(2. / 16., 2. / 16.);

        #[repr(C)]
        #[derive(Debug, Zeroable, Pod, Clone, Copy)]
        struct Mat3 {
            columns: [[f32; 4]; 3],
        }

        let affine_mats = (0..16)
            .into_iter()
            .flat_map(|col| {
                (0..16).into_iter().map(move |row| {
                    let tx: f32 = -1. + 2. * (col as f32 * 1. / 16.) + (1. / 16.);
                    let ty: f32 = -1. + 2. * (row as f32 * 1. / 16.) + (1. / 16.);

                    Mat3 {
                        columns: [[size, 0., 0., 0.], [0., size, 0., 0.], [tx, ty, 1., 0.]],
                    }
                })
            })
            .collect::<Vec<_>>();
        // TODO: Don't recreate buffer each frame, try only write via queue.
        // This likely avoids a GPU alloc each frame?
        let affine_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex08-uni-projs".into(),
            contents: bytemuck::cast_slice(&affine_mats),
            usage: BufferUsages::UNIFORM,
        });

        let tm = TimeMouse {
            _time: self.common.time.as_secs_f32(),
            _mouse: e.mouse_window_space(),
        };

        let time_mouse_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex08-uni-time-mouse".into(),
            contents: bytemuck::cast_slice(&[tm]),
            usage: BufferUsages::UNIFORM,
        });

        let tws = self
            .textures
            .iter()
            .map(|t| t.create_view(&TextureViewDescriptor::default()))
            .collect::<Vec<_>>();
        let tws_refs: Vec<&wgpu::TextureView> = tws.iter().collect();

        let bg0: wgpu::BindGroup = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex08-bg0".into(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: time_mouse_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: affine_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureViewArray(&tws_refs),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            layout: &self.bgl0,
        });

        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex08-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.,
                        }),
                        store: true,
                    },
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_bind_group(0, &bg0, &[]);
            rpass.set_vertex_buffer(0, self.quad.slice(..));
            rpass.draw(0..6, 0..256);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
