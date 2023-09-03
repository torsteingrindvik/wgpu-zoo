/*
Goals:
    - Draw to a storage texture
    - Copy it to framebuffer
        - UPDATE: We ended up doing double buffering and swapping the role of the two textures each frame.
            The roles are storage (write only) and sampling a texture (read only).
    - Display it

Learned:
    - A render pass must have some attachment (color/depth), so we can't draw to a storage texture then copy that into the frame
        * Having an array with a single `None` entry doesn't help
    - We cannot do a texture copy from storage texture to framebuffer if that goes from R32float to Bgra8UnormSrbg
    - We can bind a texture as storage and write to it then bind it as a normal texture the next frame and read from it
    - If we use the moving lines, the previous frame will have written to storage, then the next frame will have advanced the fragments
        that will sample the previous frames.
        Those fragments will then not align, so we get flickering.
        More flickering on the outer edges because those fragments have advanced the most.
    - Issues around texture format incompatibility (like copy texture to texture requiring same format (except srgb-ness?)) can be avoided
        by simply _not_ doing a copy but using a sampler to read from one, then using the sampled value to store into the other.
 */
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, CommandEncoderDescriptor, FragmentState,
    ImageSubresourceRange, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension,
    TextureUsages, TextureViewDescriptor, VertexState,
};

use crate::{util::ExampleCommonState, Example, ExampleData};

pub struct Example07 {
    common: ExampleCommonState,
    render_pipeline: Option<RenderPipeline>,
    bgl0: BindGroupLayout,
    sampler: Sampler,
    textures: [Texture; 2],
}

impl Example for Example07 {
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

impl Example07 {
    pub fn new(e: &ExampleData) -> Self {
        let shader_source = "ex07.wgsl";
        let texture_format = e.swapchain_format;
        let common = ExampleCommonState::new(&e.device, texture_format, shader_source, "ex07");

        // Remove suffix because else we get
        //  > Texture usages TextureUsages(STORAGE_BINDING) are not allowed on a texture of type Bgra8UnormSrgb
        //
        // UPDATE: Can't use this format as a storage texture anyway
        //
        // let storage_texture_format = texture_format.remove_srgb_suffix();

        let storage_texture_format = wgpu::TextureFormat::R32Float;

        let bgl0 = e
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: "ex07-bgl0".into(),
                entries: &[
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
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: storage_texture_format,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let textures = [
            e.device.create_texture(&TextureDescriptor {
                label: "ex07-texture".into(),
                size: e.extent_3d(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: storage_texture_format,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
            e.device.create_texture(&TextureDescriptor {
                label: "ex07-texture2".into(),
                size: e.extent_3d(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: storage_texture_format,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        ];

        let sampler = e.device.create_sampler(&SamplerDescriptor::default());

        Self {
            render_pipeline: None,
            common,
            bgl0,
            textures,
            sampler,
        }
    }

    fn make_render_pipeline(&self, e: &ExampleData) -> RenderPipeline {
        let texture_format = e.swapchain_format;

        e.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: "ex07-rpassd".into(),
            layout: Some(&e.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: "ex07-rpass-pld".into(),
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
                topology: wgpu::PrimitiveTopology::TriangleStrip,
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
            label: "ex07-ce".into(),
        });

        let texture_sampled = self.common.frame() as usize % 2;
        let texture_storage = (self.common.frame() as usize + 1) % 2;

        if self.common.dirty || self.render_pipeline.is_none() {
            self.common.dirty = false;
            self.render_pipeline = Some(self.make_render_pipeline(&e));

            ce.clear_texture(
                &self.textures[texture_storage],
                &ImageSubresourceRange::default(),
            );

            ce.clear_texture(
                &self.textures[texture_sampled],
                &ImageSubresourceRange::default(),
            );
        }

        // Render pass resources
        let current_texture = e.surface.get_current_texture().unwrap();
        let screen_view = &current_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        let time_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex07-uni-time".into(),
            contents: self.common.time.as_secs_f32().to_le_bytes().as_ref(),
            usage: BufferUsages::UNIFORM,
        });
        let mouse_buf = e.device.create_buffer_init(&BufferInitDescriptor {
            label: "ex07-uni-mouse".into(),
            contents: bytemuck::cast_slice(e.mouse_window_space().as_slice()),
            usage: BufferUsages::UNIFORM,
        });

        let bg0: wgpu::BindGroup = e.device.create_bind_group(&BindGroupDescriptor {
            label: "ex07-bg0".into(),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: time_buf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &self.textures[texture_sampled]
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &self.textures[texture_storage]
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: mouse_buf.as_entire_binding(),
                },
            ],
            layout: &self.bgl0,
        });

        {
            let mut rpass = ce.begin_render_pass(&RenderPassDescriptor {
                label: "ex07-rp".into(),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: screen_view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                // todo
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline.as_ref().unwrap());
            rpass.set_bind_group(0, &bg0, &[]);
            rpass.draw(0..4, 0..1);
        }

        e.queue.submit(std::iter::once(ce.finish()));
        current_texture.present();
    }
}
