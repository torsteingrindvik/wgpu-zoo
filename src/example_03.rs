use std::{f32::consts::TAU, time::Duration};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
    ComputePipeline, ComputePipelineDescriptor, FragmentState, MultisampleState, Operations,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexState,
};
use winit::event::VirtualKeyCode;

use crate::{Example, ExampleData};

pub struct Example03 {
    render_pipeline: Option<RenderPipeline>,
    compute_pipeline: ComputePipeline,
    bgl0: BindGroupLayout,
    vertices: [[f32; 2]; 3],
    num_instances: u32,
    time: Duration,
    radius: f32,
    polygon_mode: PolygonMode,
    shader_module: ShaderModule,
    gon: u16,
    gon_buf: Buffer,
    goni_buf: Buffer,
}

// Make an n-gon via the resolution n.
// The first vertex returned is centered at [0., 0].
// The rest are points on the unit circle separated by an appropriate angle.
// A fitting index buffer is also given for rendering as a triangle list.
fn make_ngon(n: u16) -> (Vec<[f32; 2]>, Vec<u16>) {
    assert!(n > 1);
    let mut gon = vec![[0., 0.]];

    // So if n = 3, then we end up with
    // a total of 4 vertices:
    //  [0., 0.],
    //  [1., 0.],
    //  [0., 1.],
    //  [-1., 0.],
    //  [0., -1.],
    for i in 0..=n {
        let (y, x) = (i as f32 / (n + 1) as f32 * TAU).sin_cos();
        gon.push([x, y]);
    }

    // With the above example, we want to make 3 triangles, using vertices:
    //  [0, 1, 2],
    //  [0, 2, 3],
    //  [0, 3, 4],
    let mut indices = vec![];
    for i in 0..n {
        // Center vertex
        indices.push(0);
        // New vertices
        indices.push(i + 1);
        indices.push(i + 2);
    }

    (gon, indices)
}

#[test]
fn make_3gon() {
    let (vertices, indices) = make_ngon(3);
    dbg!(&vertices, &indices);
}

impl Example for Example03 {
    fn handle_key(&mut self, key: winit::event::VirtualKeyCode) {
        // Use Up/Down to switch between polygon modes.
        // Remove the render pipeline such that it's recreated
        // later (needed to apply new mode).
        match key {
            VirtualKeyCode::Up | VirtualKeyCode::W => {
                self.polygon_mode = match self.polygon_mode {
                    PolygonMode::Fill => PolygonMode::Fill,
                    PolygonMode::Line => PolygonMode::Fill,
                    PolygonMode::Point => PolygonMode::Line,
                };
                self.render_pipeline = None;
            }
            VirtualKeyCode::Down | VirtualKeyCode::S => {
                self.polygon_mode = match self.polygon_mode {
                    PolygonMode::Fill => PolygonMode::Line,
                    PolygonMode::Line => PolygonMode::Point,
                    PolygonMode::Point => PolygonMode::Point,
                };
                self.render_pipeline = None;
            }
            VirtualKeyCode::A => {
                self.radius = (self.radius - 0.1).max(0.1);
            }
            VirtualKeyCode::D => {
                self.radius = (self.radius + 0.1).min(2.);
            }
            _ => {}
        }
    }

    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn dt(&mut self, dt: Duration) {
        self.time += dt;
    }

    fn handle_scroll(&mut self, scroll_up: bool) {
        if scroll_up {
            self.num_instances = (self.num_instances + 1).min(100);
        } else {
            self.num_instances = self.num_instances.saturating_sub(1).max(3);
        }
        dbg!(&self.num_instances);
    }
}

impl Example03 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_module = e.device.create_shader_module(ShaderModuleDescriptor {
            label: "ex03-sm".into(),
            source: wgpu::ShaderSource::Wgsl(include_str!("ex03.wgsl").into()),
        });

        let shader_module_compute = e.device.create_shader_module(ShaderModuleDescriptor {
            label: "ex03-sm-compute".into(),
            source: wgpu::ShaderSource::Wgsl(include_str!("ex03-compute.wgsl").into()),
        });

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex03-bgld0".into(),
                entries: &[
                    // Total # of instances
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Radius
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
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

        let polygon_mode = PolygonMode::Fill;

        let compute_pipeline = e
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("ex03-cpassd"),
                layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: "ex03-cpass-pld".into(),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                })),
                module: &shader_module_compute,
                entry_point: "cs",
            });

        let gon = 10;
        let (vertices, indices) = make_ngon(gon);
        let gon_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("gonbuf"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: BufferUsages::VERTEX,
        });
        let goni_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("gonbuf-indices"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: BufferUsages::INDEX,
        });

        Self {
            render_pipeline: None,
            vertices: [[-0.5, 0.0], [0.0, 1.0], [0.5, 0.0]],
            time: Duration::from_secs(0),
            bgl0,
            num_instances: 10,
            radius: 0.3,
            polygon_mode,
            shader_module,
            compute_pipeline,
            gon_buf,
            goni_buf,
            gon,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex03-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex03-rpass-pld".into(),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &self.shader_module,
                entry_point: "vs",
                // todo: query set later and swap order and see if there is a diff?
                buffers: &[
                    // The triangle vertices
                    VertexBufferLayout {
                        array_stride: wgpu::VertexFormat::Float32x2.size(),
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[],
                    },
                ],
            },
            fragment: Some(FragmentState {
                module: &self.shader_module,
                entry_point: "fs",
                // what if several targets? just have to match in render pass?
                targets: &[Some(e.swapchain_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: self.polygon_mode,
                ..Default::default()
            },
            // todo: enable and see
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    // fn vertices(&self) -> &[u8] {
    //     bytemuck::cast_slice(&self.vertices)
    // }

    pub fn do_render(&mut self, e: &ExampleData) {
        // let index_buf = e.device.create_buffer_init(&BufferInitDescriptor {
        //     label: "ex03-index-buf".into(),
        //     contents: bytemuck::cast_slice(self.vertices()),
        //     usage: BufferUsages::VERTEX,
        // });

        // let num_instances_buf = e.device.create_buffer_init(&BufferInitDescriptor {
        //     label: "ex03-uni-ninst".into(),
        //     contents: self.num_instances.to_le_bytes().as_ref(),
        //     usage: BufferUsages::UNIFORM,
        // });
        // let radius_buf = e.device.create_buffer_init(&BufferInitDescriptor {
        //     label: "ex03-uni-radius".into(),
        //     contents: self.radius.to_le_bytes().as_ref(),
        //     usage: BufferUsages::UNIFORM,
        // });
        // let mouse_buf = e.device.create_buffer_init(&BufferInitDescriptor {
        //     label: "ex03-uni-mouse".into(),
        //     contents: bytemuck::cast_slice(e.mouse.as_slice()),
        //     usage: BufferUsages::UNIFORM,
        // });
        // let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
        //     label: "ex03-uni-time".into(),
        //     contents: self.time.as_secs_f32().to_le_bytes().as_ref(),
        //     usage: BufferUsages::UNIFORM,
        // });

        // Command encoder begin

        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex03-ce".into(),
        });

        // Compute pass resources

        // Compute pass
        {
            let mut cpass = ce.begin_compute_pass(&ComputePassDescriptor {
                label: "ex03-cp".into(),
            });

            // cpass.set_pipeline();
        }

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        // let bg0 = e.device.create_bind_group(&BindGroupDescriptor {
        //     label: "ex03-bg-0".into(),
        //     layout: &self.bgl0,
        //     entries: &[
        //         BindGroupEntry {
        //             binding: 0,
        //             resource: num_instances_buf.as_entire_binding(),
        //         },
        //         BindGroupEntry {
        //             binding: 1,
        //             resource: radius_buf.as_entire_binding(),
        //         },
        //         BindGroupEntry {
        //             binding: 2,
        //             resource: mouse_buf.as_entire_binding(),
        //         },
        //         BindGroupEntry {
        //             binding: 3,
        //             resource: time_buf.as_entire_binding(),
        //         },
        //     ],
        // });

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

            if self.render_pipeline.is_none() {
                self.render_pipeline = Some(self.make_render_pipeline(e));
            }

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_vertex_buffer(0, self.gon_buf.slice(..));
            rpass.set_index_buffer(self.goni_buf.slice(..), wgpu::IndexFormat::Uint16);
            // rpass.set_bind_group(0, &bg0, &[]);
            rpass.draw(0..self.gon as u32, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
