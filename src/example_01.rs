use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, CommandEncoderDescriptor, Device, FragmentState, MultisampleState, Operations,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, TextureFormat,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexState,
};
use winit::event::VirtualKeyCode;

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example01 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    vertices: [[f32; 2]; 3],
}

impl Example for Example01 {
    fn handle_key(&mut self, key: winit::event::VirtualKeyCode) {
        match key {
            VirtualKeyCode::Up => {
                self.update_vertices(0.0, 0.1);
            }
            VirtualKeyCode::Down => {
                self.update_vertices(0.0, -0.1);
            }
            VirtualKeyCode::Left => {
                self.update_vertices(-0.1, 0.0);
            }
            VirtualKeyCode::Right => {
                self.update_vertices(0.1, 0.0);
            }
            _ => {}
        }
    }

    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }
}

fn render_pipeline(
    device: &Device,
    shader_module: &ShaderModule,
    texture_format: TextureFormat,
    polygon_mode: PolygonMode,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: "ex01-rpd".into(),
        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: "ex01-pld".into(),
            bind_group_layouts: &[],
            // todo: test this
            push_constant_ranges: &[],
        })),
        vertex: VertexState {
            module: shader_module,
            entry_point: "vs",
            // todo: try indexing
            buffers: &[VertexBufferLayout {
                // how far between elements
                array_stride: wgpu::VertexFormat::Float32x2.size(),

                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    // todo: match binding location in shader
                    shader_location: 0,
                }],
            }],
        },
        fragment: Some(FragmentState {
            module: shader_module,
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

impl Example01 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex01.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex01");

        Self {
            render_pipeline: None,
            vertices: [[-0.5, 0.0], [0.0, 1.0], [0.5, 0.0]],
            common,
        }
    }

    pub fn update_vertices(&mut self, offset_x: f32, offset_y: f32) {
        self.vertices[0][0] += offset_x;
        self.vertices[1][0] += offset_x;
        self.vertices[2][0] += offset_x;

        self.vertices[0][1] += offset_y;
        self.vertices[1][1] += offset_y;
        self.vertices[2][1] += offset_y;
    }

    fn vertices(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    fn do_render(&mut self, e: &ExampleData) {
        if self.common.dirty || self.render_pipeline.is_none() {
            self.render_pipeline = Some(render_pipeline(
                &e.device,
                &self.common.shader_module,
                self.common.texture_format,
                self.common.polygon_mode,
            ));
            self.common.dirty = false;
        }

        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex01-ce".into(),
        });

        let b: wgpu::Buffer = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex01-buf".into(),
            contents: bytemuck::cast_slice(self.vertices()),
            usage: BufferUsages::VERTEX,
        });

        let current_texture = match e.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Outdated) => return,
            Err(e) => panic!("{e:?}"),
        };
        let view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex01-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_vertex_buffer(0, b.slice(..));
            rpass.draw(0..self.vertices.len() as u32, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
