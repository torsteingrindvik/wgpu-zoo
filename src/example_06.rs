/*
Goals:
    - Try out setting the viewport- try rendering the same thing four times (once for each quadrant).
        Try making the mouse decide where the quad split goes.

Learned:
    - A viewport must have depth range set to [0. -> 1.]
    - A viewport must have >0 width and height
    - The difference between setting the viewport and using scissor rect is that scissor rect discards fragments
        outside the rect, but viewport rect resizes a whole window to fit the new viewport.
 */
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, CommandEncoderDescriptor, FragmentState, MultisampleState,
    Operations, PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages,
    TextureViewDescriptor, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example06 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    bgl0: BindGroupLayout,
}

impl Example for Example06 {
    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }
}

impl Example06 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex06.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex06");

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex06-bgl0".into(),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        Self {
            render_pipeline: None,
            common,
            bgl0,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        let texture_format = e.swapchain_format;

        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex06-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex06-rpass-pld".into(),
                bind_group_layouts: &[&self.bgl0],
                push_constant_ranges: &[],
            })),
            vertex: VertexState {
                module: &self.common.shader_module,
                entry_point: "vs",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &self.common.shader_module,
                entry_point: "fs",
                targets: &[Some(texture_format.into())],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                polygon_mode: self.common.polygon_mode,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn do_render(&mut self, e: &ExampleData) {
        if self.common.dirty || self.render_pipeline.is_none() {
            self.common.dirty = false;
            self.render_pipeline = Some(self.make_render_pipeline(&e));
        }

        // Command encoder begin
        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex06-ce".into(),
        });

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let screen_view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex06-uni-time".into(),
            contents: self.common.time.as_secs_f32().to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });
        let bg0 = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex06-bg0".into(),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: time_buf.as_entire_binding(),
            }],
            layout: &self.bgl0,
        });

        let extent3d = e.extent_3d();
        let (width, height) = (extent3d.width, extent3d.height);
        let [mouse_x, mouse_y] = e.mouse_window_space();

        let [mx, my] = [mouse_x.max(1) as f32, mouse_y.max(1) as f32];
        let (w, h) = (width as f32, height as f32);

        let quadrants = [
            // Top left quadrant
            [0., 0., mx, my],
            // Top right quadrant
            [mx, 0., w - mx, my],
            // Bottom left quadrant
            [0., my, mx, h - my],
            // Bottom right quadrant
            [mx, my, w - mx, h - my],
        ];

        for (idx, [x, y, w, h]) in quadrants.into_iter().enumerate() {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex06-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    // Default: Clear on load, and then store
                    ops: if idx == 0 {
                        Operations::default()
                    } else {
                        Operations {
                            // Only clear first pass
                            load: wgpu::LoadOp::Load,
                            ..Default::default()
                        }
                    },
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_viewport(x, y, w, h, 0., 1.);
            rpass.set_bind_group(0, &bg0, &[]);
            rpass.draw(0..64, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
