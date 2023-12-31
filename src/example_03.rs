/*
Goals:
    INITIAL
    - Try triangle strip
        - Done for quad
    - Try visually showing mouse close to a vertex
        - Done via frag shader
    - Try passing the threshold for how close mouse must be to show vertex proximity via push constant
        - Done

    MORE FANCY
    - When click + near, should attach to vertex somehow to be able to swap
        - Done

Things we learned:
    - The builtin position is transformed into another coord space when moving from vertex to frag shaders.
    If we want to relate things like mouse (clip space), we then have to do some math
    - An array cannot have stride length 8 (e.g. array<vec2<f32>> will fail), it must have alignment 16
    - An array should be array<vec4<f32>, 4> to have 4 elements, then it gets the SIZED flag
    - It's hard to think in terms of single fragments vs. the whole frag shader
 */
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, CommandEncoderDescriptor, FragmentState, MultisampleState,
    Operations, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example03 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    bgl0: BindGroupLayout,
    vertices: [[f32; 2]; 4],

    // Index of selected vertex if any
    selected_vertex: Option<u32>,

    mouse_close_threshold: f32,
}

impl Example for Example03 {
    fn handle_key(&mut self, _key: winit::event::VirtualKeyCode) {}

    fn handle_scroll(&mut self, scroll_up: bool) {
        if scroll_up {
            self.mouse_close_threshold = (self.mouse_close_threshold + 0.01).min(0.3);
        } else {
            self.mouse_close_threshold = (self.mouse_close_threshold - 0.01).max(0.1);
        }
    }

    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }

    fn handle_click(&mut self, position: [f32; 2], pressed: bool) {
        dbg!(pressed, position);

        if pressed {

        for (i, v) in self.vertices.iter().enumerate() {
            let len = 
                // Length of vertex index i to mouse
                ((position[0] - v[0]).powf(2.0) + (position[1] - v[1]).powf(2.0)).sqrt();
            let close_enough = len < self.mouse_close_threshold;
            dbg!(
                i,
                len,
                close_enough
            );

            if close_enough {
                self.selected_vertex = Some(i as u32);
            }
        }
        } else {
            self.selected_vertex = None;
        }

    }
}

impl Example03 {
    fn vertices_align16(&self) -> [[f32; 4]; 4] {
        [
            [self.vertices[0][0], self.vertices[0][1], 0.0, 0.0],
            [self.vertices[1][0], self.vertices[1][1], 0.0, 0.0],
            [self.vertices[2][0], self.vertices[2][1], 0.0, 0.0],
            [self.vertices[3][0], self.vertices[3][1], 0.0, 0.0],
        ]
    }

    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex03.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex03");

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex03-bgld0".into(),
                entries: &[
                    // Viewport
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Quad
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Mouse
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Time
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Triangle strip: https://docs.rs/wgpu/latest/wgpu/enum.PrimitiveTopology.html#variant.TriangleStrip
        // Says vertices 0 1 2 3 will lead to two triangles:
        //
        //  - 0 1 2
        //  - 2 1 3
        //
        // So then we make a quad by assigning vertex indices as such:
        //  0 = top left
        //  1 = bottom left
        //  2 = top right
        //  3 = bottom right
        let vertices = [[-0.5, 0.5], [-0.5, -0.5], [0.5, 0.5], [0.5, -0.5]];


        Self {
            render_pipeline: None,
            bgl0,
            vertices,
            common,
            mouse_close_threshold: 0.2,
            selected_vertex: None,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex03-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex03-rpass-pld".into(),
                bind_group_layouts: &[&self.bgl0],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::FRAGMENT,
                    range: 0..4,
                }],
            })),
            vertex: VertexState {
                module: &self.common.shader_module,
                entry_point: "vs",
                buffers: &[
                    // The triangle vertices
                    VertexBufferLayout {
                        array_stride: wgpu::VertexFormat::Float32x2.size(),
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &self.common.shader_module,
                entry_point: "fs",
                // what if several targets? just have to match in render pass?
                targets: &[Some(e.swapchain_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                polygon_mode: self.common.polygon_mode,
                ..Default::default()
            },
            // todo: enable and see
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn do_render(&mut self, e: &ExampleData) {
        if self.common.dirty || self.render_pipeline.is_none() {
            self.render_pipeline = Some(self.make_render_pipeline(&e));
            self.common.dirty = false;
        }
        
        // If something is selected, move that vertex to where the mouse is
        if let Some(vi) = self.selected_vertex {
            self.vertices[vi as usize] = e.mouse_clip_space();
        }

        let viewport_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex03-uni-viewport".into(),
            contents: bytemuck::cast_slice(e.viewport.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let quad_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex03-uni-quad".into(),
            contents: bytemuck::cast_slice(self.vertices_align16().as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let mouse_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex03-uni-mouse".into(),
            contents: bytemuck::cast_slice(e.mouse_clip_space().as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex03-uni-time".into(),
            contents: self.common.time.as_secs_f32().to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });

        // Command encoder begin
        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex03-ce".into(),
        });

        let quad = e.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("ex03-quad"),
            contents: bytemuck::cast_slice(self.vertices.as_slice()),
            usage: BufferUsages::VERTEX,
        });

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        let bg0 = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex03-bg-0".into(),
            layout: &self.bgl0,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: viewport_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: quad_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: mouse_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: time_buf.as_entire_binding(),
                },
            ],
        });

        // Render pass
        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex03-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_vertex_buffer(0, quad.slice(..));
            rpass.set_bind_group(0, &bg0, &[]);

            rpass.set_push_constants(
                ShaderStages::FRAGMENT,
                0,
                self.mouse_close_threshold.to_le_bytes().as_ref(),
            );
            rpass.draw(0..4, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
