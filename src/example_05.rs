/*
Goals:
    - Trying out MSAA, similar to the wgpu MSAA line example.
    - Trying `LineList`
    - Trying scissor rect

Things we learned:
    - `TextureUsage::empty()` is not ok for the MSAA texture.
        I thought perhaps that would be OK because the WGPU example stated we don't need to store the result in the renderpass
        since we only use the resolved one.

    - Choosing 16 as the sample count doesn't work out of the box. The error message from WGPU was great:
        > Sample count 16 is not supported by format Bgra8UnormSrgb on this device.
        > It may be supported by your adapter through the TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES feature.

    - 16 was not available even though the above feature was enabled.
        By checking the texture format feature flags we see that sample count 8 should work.

    - Scissor rect depends on whether it is called before or after the render pass draw- it only acts on subsequent draw calls

    - Scissor rect takes (x, y) (u32, u32) as a an offset from the top left corner.
        If e.g. (100, 50) is passed, the first 100 pixels horizontally are not drawn, as well as the first 50 vertical pixels.
    - Scissor rect takes  (width, height) (u32, u32) too, which is in viewport pixels, and determine where we will end up drawing.
    - Scissor rect offset+width/height must not fall outside the thing we resolve into

    - There is no problem in drawing only the left hand side using scissor rect in one render pass, then drawing the right hand side using a different pipeline
        but to the same texture (the swapchain) in another pass before presenting.
        This allows us to show MSAA on one side and non-MSAA on the other side.
 */
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, CommandEncoderDescriptor, FragmentState, MultisampleState,
    Operations, PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, Texture,
    TextureDescriptor, TextureUsages, TextureViewDescriptor, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example05 {
    common: ExampleCommonState,
    render_pipeline_msaa: Option<RenderPipeline>,
    render_pipeline: Option<RenderPipeline>,
    msaa_texture: Texture,
    sample_count: u32,
    bgl0: BindGroupLayout,
}

impl Example for Example05 {
    fn render(&mut self, data: &ExampleData) {
        self.do_render(data);
    }

    fn common(&mut self) -> &mut ExampleCommonState {
        &mut self.common
    }
}

impl Example05 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex05.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex05");
        let sample_count = e.max_sample_count;
        let msaa_texture = e.device.create_texture(&TextureDescriptor {
            label: "MSAA".into(),
            size: e.extent_3d(),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            // Is this ok?
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex05-bgl0".into(),
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
            render_pipeline_msaa: None,
            render_pipeline: None,
            common,
            msaa_texture,
            sample_count,
            bgl0,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData, multisample: bool) -> RenderPipeline {
        let texture_format = e.swapchain_format;

        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex05-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex05-rpass-pld".into(),
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
            // Here we go
            multisample: MultisampleState {
                count: if multisample { self.sample_count } else { 1 },
                ..Default::default()
            },
            multiview: None,
        })
    }

    pub fn do_render(&mut self, e: &ExampleData) {
        if self.common.dirty || self.render_pipeline.is_none() {
            self.common.dirty = false;
            self.render_pipeline_msaa = Some(self.make_render_pipeline(&e, true));
            self.render_pipeline = Some(self.make_render_pipeline(&e, false));
        }

        // Command encoder begin
        let mut ce = e.device.create_command_encoder(&CommandEncoderDescriptor {
            label: "ex05-ce".into(),
        });

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let screen_view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());
        let msaa_view = self
            .msaa_texture
            .create_view(&TextureViewDescriptor::default());

        let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex05-uni-time".into(),
            contents: self.common.time.as_secs_f32().to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });
        let bg0 = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex05-bg0".into(),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: time_buf.as_entire_binding(),
            }],
            layout: &self.bgl0,
        });

        let extent3d = e.extent_3d();
        let (width, height) = (extent3d.width, extent3d.height);

        // Render pass 1: MSAA left side
        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex05-rp-msaa".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &msaa_view,
                    resolve_target: Some(screen_view),
                    // Default: Clear on load, and then store
                    ops: Operations::default(),
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline_msaa.as_ref().unwrap());
            rpass.set_bind_group(0, &bg0, &[]);
            // Draw left half
            rpass.set_scissor_rect(0, 0, width / 2, height);
            rpass.draw(0..64, 0..1);
        }

        // Render pass 2: Non-MSAA right side
        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex05-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &screen_view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_bind_group(0, &bg0, &[]);
            // Draw right half
            rpass.set_scissor_rect(width / 2, 0, width / 2, height);
            rpass.draw(0..64, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
