use std::path::Path;

pub fn create_shader(
    device: &wgpu::Device,
    path: impl AsRef<Path>,
) -> Result<wgpu::ShaderModule, Box<dyn std::error::Error>> {
    let data = std::fs::read(path.as_ref())?;

    let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        flags: wgpu::ShaderFlags::all(),
        label: Some(&path.as_ref().to_string_lossy()),
        source: wgpu::util::make_spirv(&data),
    });

    Ok(module)
}
