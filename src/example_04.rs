/*
Goals:
    - See if we can make sense of having several color attachments

Things we learned:
    - vec4 in vertex shader w param thing:
        If we in wgsl place a vertex using vec4(-1., -1., 0., 1.) * 0.5,
        this vertex will be in the bottom left corner. It ignores the 0.5 factor, since that
        also applies to the last param `w`.

        Likely due to some 1/w thing happening when moving to the frag shader.

    - vec4 with w == 0:
        If we make a vertex via vec4(-1., -1., 0., 0.), we won't get any triangle (when doing the same for other w values).

    - We can use the window's view twice (alias it?).
        If we use two color targets and both use the winit window texture as view, we don't get any complaints.
        It seems that then the second location overwrites the first.

    - We need to handle resize if we render offline (TODO).
 */
use wgpu::{
    ColorWrites, CommandEncoderDescriptor, Extent3d, FragmentState, MultisampleState, Operations,
    PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Texture, TextureDescriptor, TextureUsages,
    TextureViewDescriptor, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example04 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    offscreen: Texture,
}

impl Example for Example04 {
    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }
}

impl Example04 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex04.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex04");

        let offscreen = e.device.create_texture(&TextureDescriptor {
            label: "offscreen".into(),
            size: Extent3d {
                width: e.viewport[0] as u32,
                height: e.viewport[1] as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self {
            render_pipeline: None,
            common,
            offscreen,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        let texture_format = e.swapchain_format;

        let cts1 = wgpu::ColorTargetState {
            format: texture_format,
            blend: None,
            write_mask: ColorWrites::all(),
        };
        let cts2 = wgpu::ColorTargetState {
            format: texture_format,
            blend: None,
            // See ex04.wgsl
            write_mask: ColorWrites::GREEN | ColorWrites::BLUE,
        };

        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex04-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex04-rpass-pld".into(),
                bind_group_layouts: &[],
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
                // This is what we want to poke at: We now have more than one of these
                targets: &[Some(cts1), Some(cts2)],
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
        if self.common.dirty || self.render_pipeline.is_none() {
            self.render_pipeline = Some(self.make_render_pipeline(&e));
            self.common.dirty = false;
        }

        // Command encoder begin
        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex04-ce".into(),
        });

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let screen_view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        // To actually see the output of this use renderdoc.
        // (Since we don't save it to disk)
        let offscreen_view = self
            .offscreen
            .create_view(&TextureViewDescriptor::default());

        // Render pass
        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex04-rp".into(),
                // The color attachments must match the render pipeline's fragment state targets.
                // Since that has `Some(_), Some(_)`, we crash if we have e.g. `Some(_), None` here.
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &screen_view,
                        resolve_target: None,
                        // Default: Clear on load, and then store
                        ops: Operations::default(),
                    }),
                    Some(RenderPassColorAttachment {
                        view: &offscreen_view,
                        resolve_target: None,
                        ops: Operations::default(),
                    }),
                ],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            // No vertex buffer, so we'll use the trick where we calc a triangle from the indices within
            // the 0..3 range instead
            rpass.draw(0..3, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
