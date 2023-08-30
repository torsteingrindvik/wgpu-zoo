use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, CommandEncoderDescriptor, Device, FragmentState,
    MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, ShaderStages, TextureFormat, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexState,
};
use winit::event::VirtualKeyCode;

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example02 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    bgl0: BindGroupLayout,
    vertices: [[f32; 2]; 3],
    num_instances: u32,
    radius: f32,
}

impl Example for Example02 {
    fn handle_key(&mut self, key: winit::event::VirtualKeyCode) {
        // Use Up/Down to switch between polygon modes.
        // Remove the render pipeline such that it's recreated
        // later (needed to apply new mode).
        match key {
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

    fn handle_scroll(&mut self, scroll_up: bool) {
        if scroll_up {
            self.num_instances = (self.num_instances + 1).min(100);
        } else {
            self.num_instances = self.num_instances.saturating_sub(1).max(3);
        }
        dbg!(&self.num_instances);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }
}

fn render_pipeline(
    device: &Device,
    shader_module: &ShaderModule,
    bgl: &BindGroupLayout,
    texture_format: TextureFormat,
    polygon_mode: PolygonMode,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: "ex02-rpd".into(),
        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: "ex02-pld".into(),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        })),
        vertex: VertexState {
            module: &shader_module,
            entry_point: "vs",
            // todo: query set later and swap order and see if there is a diff?
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
            module: &shader_module,
            entry_point: "fs",
            // what if several targets? just have to match in render pass?
            targets: &[Some(texture_format.into())],
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode,
            ..Default::default()
        },
        // todo: enable and see
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    })
}

impl Example02 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex02.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex02");

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex02-bgld0".into(),
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

        Self {
            render_pipeline: None,
            vertices: [[-0.5, 0.0], [0.0, 1.0], [0.5, 0.0]],
            bgl0,
            num_instances: 10,
            radius: 0.3,
            common,
        }
    }

    fn vertices(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn do_render(&mut self, e: &ExampleData) {
        if self.common.dirty || self.render_pipeline.is_none() {
            self.render_pipeline = Some(render_pipeline(
                &e.device,
                &self.common.shader_module,
                &self.bgl0,
                self.common.texture_format,
                self.common.polygon_mode,
            ));
            self.common.dirty = false;
        }

        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex02-ce".into(),
        });

        let index_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex02-index-buf".into(),
            contents: bytemuck::cast_slice(self.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let num_instances_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex02-uni-ninst".into(),
            contents: self.num_instances.to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });
        let radius_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex02-uni-radius".into(),
            contents: self.radius.to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });
        let mouse_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex02-uni-mouse".into(),
            contents: bytemuck::cast_slice(e.mouse.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex02-uni-time".into(),
            contents: self.common.time.as_secs_f32().to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });

        let bg0 = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex02-bg-0".into(),
            layout: &self.bgl0,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: num_instances_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: radius_buf.as_entire_binding(),
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
        let current_texture = e.surface.get_current_texture().unwrap();
        let view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex02-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_vertex_buffer(0, index_buf.slice(..));
            rpass.set_bind_group(0, &bg0, &[]);
            rpass.draw(0..self.vertices.len() as u32, 0..self.num_instances);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
