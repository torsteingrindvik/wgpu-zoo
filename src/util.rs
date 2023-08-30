use std::{borrow::Cow, path::PathBuf, time::Duration};

use wgpu::{Device, PolygonMode, ShaderModule, ShaderModuleDescriptor, TextureFormat};

/// Comman state examples should have
/// TODO: Mark dirty?
pub struct ExampleCommonState {
    pub texture_format: TextureFormat,
    pub shader_module: ShaderModule,
    pub shader_source: &'static str,
    pub label: &'static str,
    pub polygon_mode: PolygonMode,

    // How long has this example gotten to run?
    // Pauses when example inactive
    pub time: Duration,

    // Should this example recreate resources?
    // E.g. the `RenderPipeline`.
    pub dirty: bool,
}

// Create a shader module from a wgsl file in the "src" dir.
// E.g. a valid `wgsl` arg would be "ex01.wgsl".
fn shader_module<'l>(
    device: &Device,
    shader_source: &'static str,
    label: &'static str,
) -> ShaderModule {
    let mut path = PathBuf::new();
    path.push(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    path.push(&shader_source);
    println!("Loading shader at {path:?}");

    device.create_shader_module(ShaderModuleDescriptor {
        label: label.into(),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(std::fs::read_to_string(path).unwrap())),
    })
}

impl ExampleCommonState {
    pub fn new(
        device: &Device,
        texture_format: TextureFormat,
        shader_source: &'static str,
        label: &'static str,
    ) -> Self {
        Self {
            texture_format,
            shader_source,
            label,
            shader_module: shader_module(device, shader_source, label),
            polygon_mode: PolygonMode::Fill,
            dirty: true,
            time: Duration::from_secs(0),
        }
    }

    pub fn increase_time(&mut self, dt: Duration) {
        self.time += dt;
    }

    pub fn recreate_shader(&mut self, device: &Device) {
        self.shader_module = shader_module(device, self.shader_source, self.label)
    }
}
