use std::path::Path;
use std::borrow::{Borrow, Cow};
use std::ffi::OsStr;

pub fn create_shader(device: &wgpu::Device, path: impl AsRef<Path>) -> Result<wgpu::ShaderModule, Box<dyn std::error::Error>> {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let bytes = std::fs::read_to_string(path.as_ref())?;
    let kind = match path
        .as_ref()
        .extension()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy()
        .borrow()
    {
        "vert" => shaderc::ShaderKind::Vertex,
        "frag" => shaderc::ShaderKind::Fragment,
        _ => unimplemented!(),
    };
    let result = compiler
        .compile_into_spirv(
            &bytes,
            kind,
            &path.as_ref().file_name().unwrap().to_string_lossy(),
            "main",
            None,
        )?;

    let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        flags: wgpu::ShaderFlags::all(),
        label: Some(&path.as_ref().to_string_lossy()),
        source: wgpu::ShaderSource::SpirV(Cow::from(result.as_binary())),
    });

    Ok(module)
}