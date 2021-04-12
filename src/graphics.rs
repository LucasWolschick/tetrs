use cgmath::prelude::*;
use image::GenericImageView;
use wgpu::util::DeviceExt;

pub mod lines;
pub mod shader;
pub mod text;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl From<cgmath::Vector2<f32>> for Vertex {
    fn from(vec: cgmath::Vector2<f32>) -> Self {
        Self {
            position: [vec.x, vec.y, 0.0],
            tex_coords: [0.0, 0.0],
            color: [0.0, 0.00625, 0.025],
        }
    }
}

pub struct GraphicsState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub pipeline: wgpu::RenderPipeline,
    pub text_pipeline: wgpu::RenderPipeline,
    pub mat_buffer_bind_group: wgpu::BindGroup,
    pub mat_buffer: wgpu::Buffer,
    pub text_texture_bind_group: wgpu::BindGroup,
}

impl GraphicsState {
    pub async fn new(window: &glfw::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
            })
            .await
            .expect("Failed to get wgpu adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    limits: wgpu::Limits::default(),
                    features: wgpu::Features::NON_FILL_POLYGON_MODE,
                    label: Some("device"),
                },
                None,
            )
            .await
            .expect("Failed to get wgpu device + queue");
        let (width, height) = window.get_framebuffer_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: width as u32,
            height: height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let vertex_module = shader::create_shader(&device, "res/shaders/shader.vert.spv").unwrap();
        let fragment_module =
            shader::create_shader(&device, "res/shaders/shader.frag.spv").unwrap();

        let mat = cgmath::Matrix4::<f32>::identity();
        let raw: [[f32; 4]; 4] = mat.into();
        let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&raw),
            label: Some("mat_buffer"),
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        });
        let mat_buffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mat_buffer_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                }],
            });
        let mat_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mat_buffer.as_entire_binding(),
            }],
            label: Some("mat_buffer_bind_group"),
            layout: &mat_buffer_bind_group_layout,
        });
        let text_texture = {
            let text_texture_img = image::open("res/textures/font.png").unwrap();
            let rgba = text_texture_img.to_rgba8();
            let size = text_texture_img.dimensions();

            device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    label: Some("text_texture"),
                    mip_level_count: 1,
                    sample_count: 1,
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                },
                &rgba,
            )
        };
        let text_texture_view = text_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let text_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let text_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        count: None,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                    },
                ],
                label: Some("text_texture_bind_group_layout"),
            });
        let text_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_texture_bind_group"),
            layout: &text_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&text_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&text_texture_sampler),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[
                &mat_buffer_bind_group_layout,
                &text_texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let vblayout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    // position
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    // color
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    // tex coords
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
            ],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            label: Some("pipeline"),
            vertex: wgpu::VertexState {
                buffers: &[vblayout.clone()],
                entry_point: "main",
                module: &vertex_module,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                clamp_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                mask: !0,
                count: 1,
            },
            fragment: Some(wgpu::FragmentState {
                entry_point: "main",
                module: &fragment_module,
                targets: &[wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::REPLACE),
                    format: sc_desc.format,
                    write_mask: wgpu::ColorWrite::all(),
                }],
            }),
        });
        let text_frag_module =
            shader::create_shader(&device, "res/shaders/texquad.frag.spv").unwrap();
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            label: Some("text_pipeline"),
            vertex: wgpu::VertexState {
                buffers: &[vblayout],
                entry_point: "main",
                module: &vertex_module,
            },
            fragment: Some(wgpu::FragmentState {
                targets: &[wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    format: sc_desc.format,
                    write_mask: wgpu::ColorWrite::all(),
                }],
                entry_point: "main",
                module: &text_frag_module,
            }),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                count: 1,
                mask: !0,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                clamp_depth: false,
            },
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            pipeline,
            mat_buffer,
            mat_buffer_bind_group,
            text_pipeline,
            text_texture_bind_group,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.sc_desc.width = width;
            self.sc_desc.height = height;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        }
    }
}
