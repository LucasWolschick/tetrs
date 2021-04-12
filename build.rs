/*
    This file contains excerpts from
    https://github.com/sotrh/learn-wgpu/blob/master/code/intermediate/tutorial13-threading/build.rs,
    which is licensed under the following license:

    MIT License

    Copyright (c) 2020 Benjamin Hansen

    Permission is hereby granted,  free of charge,  to any person obtaining a copy of this
    software and associated documentation files (the "Software"),  to deal in the Software
    without  restriction,  including without limitation the rights to use,  copy,  modify,
    merge,  publish,  distribute,  sublicense,  and/or sell copies of the Software, and to
    permit  persons to whom the  Software is furnished to do so,  subject to the following
    conditions:

    The above copyright notice  and this permission notice shall be included in all copies
    or substantial portions of the Software.

    THE SOFTWARE  IS PROVIDED "AS IS",  WITHOUT WARRANTY OF ANY KIND,  EXPRESS OR IMPLIED,
    INCLUDING  BUT  NOT  LIMITED  TO  THE  WARRANTIES  OF MERCHANTABILITY,  FITNESS  FOR A
    PARTICULAR  PURPOSE  AND  NONINFRINGEMENT.  IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
    HOLDERS BE LIABLE FOR ANY CLAIM,  DAMAGES OR OTHER LIABILITY,  WHETHER IN AN ACTION OF
    CONTRACT,  TORT OR OTHERWISE,  ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
    OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

*/

use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct ShaderInfo {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

#[derive(Debug)]
enum ShaderCompilationError {
    InvalidExtension,
    ShadercInitFailure,
}

impl std::fmt::Display for ShaderCompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidExtension => "Invalid shader extension",
            Self::ShadercInitFailure => "Could not initialize shaderc compiler",
        })
    }
}

impl std::error::Error for ShaderCompilationError {}

fn load_shader(path: impl AsRef<std::path::Path>) -> Result<ShaderInfo> {
    let extension = path
        .as_ref()
        .extension()
        .ok_or(ShaderCompilationError::InvalidExtension)?
        .to_str()
        .ok_or(ShaderCompilationError::InvalidExtension)?;
    
    let kind = match extension {
        "vert" => shaderc::ShaderKind::Vertex,
        "frag" => shaderc::ShaderKind::Fragment,
        _ => return Err(Box::new(ShaderCompilationError::InvalidExtension)),
    };

    let src = std::fs::read_to_string(path.as_ref())?;
    let spv_path = path.as_ref().with_extension(format!("{}.spv", extension));

    Ok(ShaderInfo {
        src,
        src_path: path.as_ref().to_path_buf(),
        spv_path,
        kind
    })
}

fn main() -> Result<()> {
    let mut paths = [
        glob::glob("./res/shaders/*")?
    ];

    let shaders = paths.iter_mut().flatten().map(|path| load_shader(path?)).collect::<Vec<Result<_>>>().into_iter().collect::<Result<Vec<_>>>()?;

    let mut compiler = shaderc::Compiler::new().ok_or(ShaderCompilationError::ShadercInitFailure)?;

    for shader in shaders {
        println!("cargo:rerun-if-changed={}", shader.src_path.as_os_str().to_str().unwrap());

        let artifact = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            None,
        )?;

        std::fs::write(shader.spv_path, artifact.as_binary_u8())?;
    }

    Ok(())
}