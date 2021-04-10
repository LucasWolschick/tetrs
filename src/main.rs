mod game;

use std::{array, path::Path};

use cgmath::prelude::*;
use glfw::{Action, Key};
use image::GenericImageView;
use rand::prelude::*;
use wgpu::util::DeviceExt;

fn create_shader(device: &wgpu::Device, path: impl AsRef<Path>) -> Result<wgpu::ShaderModule, ()> {
    use std::borrow::{Borrow, Cow};
    use std::ffi::OsStr;
    let mut compiler = shaderc::Compiler::new().unwrap();
    let bytes = std::fs::read_to_string(path.as_ref()).map_err(|_| ())?;
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
        )
        .map_err(|e| eprintln!("{}", e))?;

    let module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        flags: wgpu::ShaderFlags::all(),
        label: Some(&path.as_ref().to_string_lossy()),
        source: wgpu::ShaderSource::SpirV(Cow::from(result.as_binary())),
    });

    Ok(module)
}

struct GraphicsState {
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
    async fn new(window: &glfw::Window) -> Self {
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
        let vertex_module = create_shader(&device, "res/shaders/shader.vert").unwrap();
        let fragment_module = create_shader(&device, "res/shaders/shader.frag").unwrap();
        
        let mat = cgmath::Matrix4::<f32>::identity();
        let raw: [[f32; 4]; 4] = mat.into();
        let mat_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&raw),
            label: Some("mat_buffer"),
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        });
        let mat_buffer_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mat_buffer_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }
                }
            ]
        });
        let mat_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: mat_buffer.as_entire_binding(),
                }
            ],
            label: Some("mat_buffer_bind_group"),
            layout: &mat_buffer_bind_group_layout,
        });
        let text_texture = {
            let text_texture_img = image::open("res/textures/font.png").unwrap();
            let rgba = text_texture_img.to_rgba8();
            let size = text_texture_img.dimensions();

            device.create_texture_with_data(&queue, &wgpu::TextureDescriptor {
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
            }, &rgba)
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
        let text_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true,
                        },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    }
                }
            ],
            label: Some("text_texture_bind_group_layout")
        });
        let text_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text_texture_bind_group"),
            layout: &text_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&text_texture_view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&text_texture_sampler)
                }
            ]
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[
                &mat_buffer_bind_group_layout,
                &text_texture_bind_group_layout
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
        let text_frag_module = create_shader(&device, "res/shaders/texquad.frag").unwrap();
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            label: Some("text_pipeline"),
            vertex: wgpu::VertexState {
                buffers: &[vblayout],
                entry_point: "main",
                module: &vertex_module
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
                mask: !0
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            }
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
            text_texture_bind_group
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.sc_desc.width = width;
            self.sc_desc.height = height;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const FIELD_WIDTH: u32 = 10;
const FIELD_HEIGHT: u32 = 20;

#[rustfmt::ignore = "readability"]
static PIECES: &[&str] = &[
    "....\
     .##.\
     .##.\
     ....",
    "..#.\
     ..#.\
     ..#.\
     ..#.",
    ".#..\
     .##.\
     ..#.\
     ....",
    "..#.\
     .##.\
     .#..\
     ....",
    ".#..\
     .#..\
     .##.\
     ....",
    "..#.\
     ..#.\
     .##.\
     ....",
    ".#..\
     .##.\
     .#..\
     ....",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    White,
}

macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        [
            ($r as f32) / 255.0,
            ($g as f32) / 255.0,
            ($b as f32) / 255.0,
        ]
    };
}

impl Color {
    fn rgb(self) -> [f32; 3] {
        match self {
            Self::Red => rgb!(221, 55, 55),
            Self::Orange => rgb!(255, 115, 25),
            Self::Yellow => rgb!(255, 215, 5),
            Self::Green => rgb!(30, 135, 30),
            Self::Blue => rgb!(0, 90, 255),
            Self::Purple => rgb!(110, 10, 225),
            Self::White => rgb!(255, 255, 255),
        }
    }
}

static PIECE_COLORS: &[Color] = {
    &[Color::Red, Color::Orange, Color::Yellow, Color::Green, Color::Blue, Color::Purple, Color::White]
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Full(Color),
}

impl Default for Cell {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Copy, Clone, Debug)]
struct Piece {
    shape: &'static str,
    color: Color,
    rot: u8,
    x: i8,
    y: i8,
}

impl Piece {
    fn new(index: usize) -> Self {
        assert!(
            index < PIECES.len(),
            "Invalid index {} given, must be in range 0-{}",
            index,
            PIECES.len()
        );
        Self {
            x: (FIELD_WIDTH / 2 - 2) as i8,
            y: 0,
            color: PIECE_COLORS[index],
            rot: 0,
            shape: PIECES[index],
        }
    }

    fn filled_at(&self, x: usize, y: usize) -> bool {
        assert!(x < 4 && y < 4, "Out of bounds index supplied");

        let i = match self.rot % 4 {
            0 => x + y * 4,
            1 => (3 - y) + x * 4,
            2 => 15 - (x + y * 4),
            3 => (3 - x) * 4 + y,
            _ => unreachable!(),
        };

        &self.shape[i..=i] == "#"
    }
}

type Field = [Cell; (FIELD_WIDTH * FIELD_HEIGHT) as usize];

struct GameState {
    /// Array containing all fixed cells
    field: Field,

    /// Active piece being manipulated by the player
    active_piece: Option<Piece>,

    /// Determines how many game ticks before the active piece is forcibly moved down
    fall_ticks: u32,

    /// Counter which
    fall_counter: u32,

    /// Determines how many game ticks fall_ticks_dec_counter starts at
    fall_accel_ticks: u32,

    /// Counter that decreases speed by 1 when it reaches 0
    fall_accel_counter: u32,

    /// Next pieces to fall
    next_pieces: Vec<Piece>,

    /// Time accumulator
    accum: f32,
    
    /// Whether we rotated last frame
    rotated: bool,

    /// Previous frame input
    last_input: PlayerInput,

    /// Current frame number
    ticker: u64,

    /// Score
    score: u64,
}

impl Default for GameState {
    fn default() -> Self {
        let mut s = Self {
            field: [Cell::Empty; (FIELD_WIDTH * FIELD_HEIGHT) as usize],
            active_piece: None,
            fall_ticks: 20,
            fall_accel_ticks: 600,
            accum: 0.0,
            rotated: false,
            last_input: PlayerInput::default(),
            ticker: 0,
            score: 0,

            // these will be set later
            fall_counter: 0,
            fall_accel_counter: 0,
            next_pieces: Vec::with_capacity(3),
        };

        s.fall_counter = s.fall_ticks;
        s.fall_accel_counter = s.fall_accel_ticks;
        let mut rand = rand::thread_rng();
        s.next_pieces.extend(std::array::IntoIter::new([
            Piece::new(rand.gen_range(0..PIECES.len())),
            Piece::new(rand.gen_range(0..PIECES.len())),
            Piece::new(rand.gen_range(0..PIECES.len())),
        ]));

        s
    }
}

fn piece_fits(piece: &Piece, field: &Field) -> bool {
    for y in 0..4 {
        for x in 0..4 {
            let rx = piece.x as isize + x;
            let ry = piece.y as isize + y;
            let offset = rx + ry * FIELD_WIDTH as isize;
            if piece.filled_at(x as usize, y as usize) {
                if offset < 0
                    || offset >= field.len() as isize
                    || rx < 0
                    || rx >= FIELD_WIDTH as isize
                    || ry < 0
                    || ry >= FIELD_HEIGHT as isize
                {
                    // out of bounds
                    return false;
                }
                
                if field[offset as usize] != Cell::Empty {
                    // filled
                    return false;
                }
            }
        }
    }

    true
}

fn add_piece(piece: &Piece, field: &mut Field) {
    for y in 0..4 {
        for x in 0..4 {
            if piece.filled_at(x as usize, y as usize) {
                let offset = (piece.x as isize + x) + (piece.y + y) as isize * FIELD_WIDTH as isize;
                if offset >= 0 && offset < field.len() as isize {
                    field[offset as usize] = Cell::Full(piece.color);
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum KeyState {
    Pressed,
    Holding,
    Released
}

impl Default for KeyState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
struct PlayerInput {
    down: KeyState,
    left: KeyState,
    right: KeyState,
    rot_right: KeyState,
    rot_left: KeyState,
}

fn input(window: &glfw::Window, last_input: &PlayerInput) -> PlayerInput {
    fn map(a: Action, prev: KeyState) -> KeyState {
        let this = match a {
            Action::Press | Action::Repeat => KeyState::Pressed,
            Action::Release => KeyState::Released,
        };

        match (this, prev) {
            (KeyState::Pressed, KeyState::Pressed) | (KeyState::Pressed, KeyState::Holding) => KeyState::Holding,
            (KeyState::Pressed, KeyState::Released) => KeyState::Pressed,
            (KeyState::Released, _) => KeyState::Released,
            _ => unreachable!(),
        }
    }

    PlayerInput {
        down: map(window.get_key(Key::Down), last_input.down),
        left: map(window.get_key(Key::Left), last_input.left),
        right: map(window.get_key(Key::Right), last_input.right),
        rot_left: map(window.get_key(Key::X), last_input.rot_left),
        rot_right: map(window.get_key(Key::Z), last_input.rot_right),
    }
}

fn update(window: &glfw::Window, state: &mut GameState, dt: std::time::Duration) {
    state.accum += dt.as_secs_f32();

    fn is_pressed(input: KeyState, ticker: u64) -> bool {
        match input {
            KeyState::Pressed => true,
            KeyState::Holding if ticker % 2 == 0 => true,
            _ => false,
        }
    }
    
    while state.accum > 0.05 {
        state.ticker += 1;

        let input = input(&window, &state.last_input);
        state.last_input = input;
        state.accum -= 0.05;

        if state.active_piece.is_none() {
            state.active_piece = Some(state.next_pieces.remove(0));
            state
                .next_pieces
                .push(Piece::new(rand::thread_rng().gen_range(0..PIECES.len())));
        }

        let mut active_piece = state.active_piece.as_mut().unwrap();

        // tick fall counter
        state.fall_counter -= 1;
        let should_fall = state.fall_counter == 0 || is_pressed(input.down, state.ticker);

        // tick down fall accelerator counter
        state.fall_accel_counter -= 1;
        if state.fall_accel_counter == 0 {
            state.fall_ticks = u32::max(state.fall_ticks - 1, 1);
            state.fall_accel_counter = state.fall_accel_ticks;
        }

        // rotate brick if requested
        if input.rot_right == KeyState::Pressed {
            if !state.rotated {
                state.rotated = true;
                let mut test_piece = active_piece.to_owned();
                test_piece.rot = (test_piece.rot + 1) % 4;
                if piece_fits(&test_piece, &state.field) {
                    active_piece.rot = test_piece.rot;
                }
            }
        } else if input.rot_left == KeyState::Pressed {
            if !state.rotated {
                state.rotated = true;
                let mut test_piece = active_piece.to_owned();
                test_piece.rot = if test_piece.rot == 0 {
                    3
                } else {
                    test_piece.rot - 1
                };
                if piece_fits(&test_piece, &state.field) {
                    active_piece.rot = test_piece.rot;
                }
            }
        } else {
            state.rotated = false;
        }

        // move brick left and right if requested
        if is_pressed(input.right, state.ticker) {
            let mut test_piece = active_piece.to_owned();
            test_piece.x += 1;
            if piece_fits(&test_piece, &state.field) {
                active_piece.x = test_piece.x;
            }
        } else if is_pressed(input.left, state.ticker) {
            let mut test_piece = active_piece.to_owned();
            test_piece.x -= 1;
            if piece_fits(&test_piece, &state.field) {
                active_piece.x = test_piece.x;
            }
        }

        // make piece fall
        if should_fall {
            state.fall_counter = state.fall_ticks;

            // verify if we can fall
            let mut test_piece = active_piece.to_owned();
            test_piece.y += 1;
            if piece_fits(&test_piece, &state.field) {
                // fall
                active_piece.y += 1;
            } else {
                // add to board
                add_piece(active_piece, &mut state.field);

                // check if any lines are deletable
                let mut deletable = Vec::new();
                'outer_loop: for y in active_piece.y..active_piece.y + 4 {
                    if y < 0 {
                        // there's nothing here; continue
                        continue;
                    }
                    if y as i32 >= FIELD_HEIGHT as i32 {
                        // we've already passed the whole board; stop
                        break;
                    }
                    for x in 0..FIELD_WIDTH {
                        let tile = state.field[x as usize + y as usize * FIELD_WIDTH as usize];
                        if tile == Cell::Empty {
                            // this line ain't it chief
                            continue 'outer_loop;
                        }
                    }
                    // if we got here this is a golden line
                    deletable.push(y);
                }
                
                if !deletable.is_empty() {
                    // add score
                    state.score += match deletable.len() {
                        1 => 1,
                        2 => 3,
                        3 => 5,
                        4 => 8,
                        _ => unreachable!()
                    } * 100;
                    
                    // delete those lines
                    for line_y in deletable {
                        for y in (0..=line_y).rev() {
                            for x in 0..FIELD_WIDTH {
                                // n^3 loop :woozy_face:
                                if y == 0 {
                                    // last line, just clear it
                                    state.field[x as usize + y as usize * FIELD_WIDTH as usize] =
                                        Cell::Empty;
                                } else {
                                    // fill it with the contents of the line above
                                    state.field[x as usize + y as usize * FIELD_WIDTH as usize] = state
                                        .field
                                        [x as usize + (y - 1) as usize * FIELD_WIDTH as usize];
                                }
                            }
                        }
                    }
                }

                // invalidate piece
                state.active_piece = None;
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coords: [f32; 2],
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

fn render_lines_pairs(positions: &[cgmath::Vector2<f32>], mut thickness: f32, index_offset: usize) -> (Vec<Vertex>, Vec<u16>) {
    use cgmath::Vector2;

    thickness /= 2.0;
    
    let mut vertices = Vec::with_capacity(positions.len() * 4);
    let mut indices = Vec::with_capacity(positions.len() * 6);

    for pair in positions.chunks_exact(2) {
        let (v1, v2) = (pair[0], pair[1]);

        /*
            v1        v2
            *---->----*
        */

        if (v2 - v1).is_zero() {
            // same vector?
            continue;
        }
        let dir = (v2 - v1).normalize();
        let across = Vector2::new(-dir.y, dir.x);

        let base_vtx = (index_offset + vertices.len()) as u16;
        vertices.extend_from_slice(&[
            (v1 + across * thickness).into(), // top left
            (v2 + across * thickness).into(), // top right
            (v1 - across * thickness).into(), // bottom left
            (v2 - across * thickness).into() // bottom right
        ]);

        // ccw should maintain in any situation
        indices.extend_from_slice(&[
            base_vtx, base_vtx + 1, base_vtx + 2, // top left triangle
            base_vtx + 1, base_vtx + 3, base_vtx + 2 // bottom right triangle
        ]);
    }

    (vertices, indices)
}

const TEXT_IMAGE_COLUMNS: i32 = 16;
const TEXT_IMAGE_ROWS: i32 = 8;
const TEXT_CHARACTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789!?@#$%\"'&()*+,-./:;<>=[]{}|\\";

fn render_text(text: &str, x: f32, y: f32, size: f32, base_idx: usize) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (i, char) in text.chars().enumerate() {
        let index = TEXT_CHARACTERS.find(char).unwrap_or((TEXT_IMAGE_COLUMNS * TEXT_IMAGE_ROWS - 1) as usize) as i32;
        let char_x = (index % TEXT_IMAGE_COLUMNS) as f32 / TEXT_IMAGE_COLUMNS as f32;
        let char_y = (index / TEXT_IMAGE_COLUMNS) as f32 / TEXT_IMAGE_ROWS as f32;
        let tile_size_x = 1.0/TEXT_IMAGE_COLUMNS as f32;
        let tile_size_y = 1.0/TEXT_IMAGE_ROWS as f32;

        let base_idx = (vertices.len() + base_idx) as u16;
        vertices.extend_from_slice(&[
            Vertex {
                position: [x + i as f32 * size, y, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [char_x, char_y],
            },
            Vertex {
                position: [x + size + i as f32 * size, y, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [char_x + tile_size_x, char_y],
            },
            Vertex {
                position: [x + i as f32 * size, y + size / 2.0, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [char_x, char_y + tile_size_y],
            },
            Vertex {
                position: [x + size + i as f32 * size, y + size / 2.0, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [char_x + tile_size_x, char_y + tile_size_y],
            },
        ]);
        indices.extend_from_slice(&[
            base_idx, base_idx + 2, base_idx + 1,
            base_idx + 1, base_idx + 2, base_idx + 3,
        ]);
    }
    
    (vertices, indices)
}

fn render(state: &GameState, graphics: &GraphicsState) -> Result<(), wgpu::SwapChainError> {
    // render fixed field
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    let inc_x = 1.0 / FIELD_WIDTH as f32;
    let inc_y = 1.0 / FIELD_HEIGHT as f32;

    // render lines
    // the reason we split our line rendering pass in two is because the X direction
    // is stretched with the global matrix. for simplicity, we render everything in
    // a single pass, which means that we need two different thickness values so the
    // lines maintain a uniform scale, with the Y thickness being half of the X thick-
    // ness. There's probably a more elegant solution out there but...
    const THICKNESS: f32 = 0.01;

    let mut vec_pairs = Vec::with_capacity((((FIELD_HEIGHT-1) + (FIELD_WIDTH-1))*2) as usize);
    for y in 1..FIELD_HEIGHT {
        vec_pairs.push(cgmath::Vector2::<f32>::new(0.0, y as f32 / FIELD_HEIGHT as f32));
        vec_pairs.push(cgmath::Vector2::<f32>::new(1.0, y as f32 / FIELD_HEIGHT as f32));
    }
    let (l_vtx, l_indx) = render_lines_pairs(&vec_pairs, THICKNESS / 2.0, vertices.len());
    vertices.extend(l_vtx);
    indices.extend(l_indx);
    vec_pairs.clear();

    for x in 1..FIELD_WIDTH {
        vec_pairs.push(cgmath::Vector2::<f32>::new(x as f32 / FIELD_WIDTH as f32, 0.0));
        vec_pairs.push(cgmath::Vector2::<f32>::new(x as f32 / FIELD_WIDTH as f32, 1.0));
    }
    let (l_vtx, l_indx) = render_lines_pairs(&vec_pairs, THICKNESS, vertices.len());
    vertices.extend(l_vtx);
    indices.extend(l_indx);
    
    // render cells
    let mut add_cell = |x: u32, y: u32, col: Color| {
        let bx = x as f32 * inc_x;
        let by = y as f32 * inc_y;

        let color = col.rgb();

        let bi = vertices.len() as u16;
        indices.extend(array::IntoIter::new([
            bi,
            bi + 1,
            bi + 2,
            bi + 2,
            bi + 1,
            bi + 3,
        ]));

        vertices.extend(array::IntoIter::new([
            Vertex {
                position: [bx, by, 0.0],
                color,
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [bx, by + inc_y, 0.0],
                color,
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [bx + inc_x, by, 0.0],
                color,
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [bx + inc_x, by + inc_y, 0.0],
                color,
                tex_coords: [0.0, 0.0],
            },
        ]));
    };

    for y in 0..FIELD_HEIGHT {
        for x in 0..FIELD_WIDTH {
            if let Cell::Full(col) = state.field[(x + y * FIELD_WIDTH) as usize] {
                add_cell(x, y, col);
            }
        }
    }

    // render active piece
    if let Some(piece) = state.active_piece {
        for y in 0..4 {
            for x in 0..4 {
                if piece.filled_at(x, y) {
                    add_cell(
                        (piece.x as i32 + x as i32) as u32,
                        (piece.y as i32 + y as i32) as u32,
                        piece.color,
                    );
                }
            }
        }
    }

    // render next pieces
    for (i, piece) in state.next_pieces.iter().enumerate() {
        for y in 0..4 {
            for x in 0..4 {
                if piece.filled_at(x, y) {
                    add_cell(
                        (x as i32 + 12) as u32,
                        (y as i32 + 2 + 5 * (i as i32)) as u32,
                        piece.color,
                    );
                }
            }
        }
    }

    // create uniforms
    let dimensions = (graphics.sc_desc.width as f32, graphics.sc_desc.height as f32);
    let aspect_ratio = dimensions.0 / dimensions.1;
    let offset = aspect_ratio / 2.0 - 0.5;
    let proj = cgmath::Matrix4::from_nonuniform_scale(0.5, 1.0, 1.0) * cgmath::ortho(-offset, 1.0 + offset, 1.0, 0.0, -1.0, 1.0);
    let raw: [[f32; 4]; 4] = proj.into();
    graphics.queue.write_buffer(&graphics.mat_buffer, 0, bytemuck::cast_slice(&raw));

    // render text
    let mut vertices_text = Vec::new();
    let mut indices_text = Vec::new();
    
    let (vt, it) = render_text(&format!("Score: {:06}", state.score), 1.1, 0.9, 0.05, vertices_text.len());
    vertices_text.extend(vt);
    indices_text.extend(it);

    let level = 20 - state.fall_ticks + 1;

    let (vt, it) = render_text(&format!("Level: {:2}", level), 1.1, 0.95, 0.05, vertices_text.len());
    vertices_text.extend(vt);
    indices_text.extend(it);

    // create buffers
    let v_buf = graphics
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertices),
            label: Some("v_buf"),
            usage: wgpu::BufferUsage::VERTEX,
        });
    let i_buf = graphics
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&indices),
            label: Some("i_buf"),
            usage: wgpu::BufferUsage::INDEX,
        });
    let v_text_buf = graphics
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vertices_text),
            label: Some("v_text_buf"),
            usage: wgpu::BufferUsage::VERTEX,
        });
    let i_text_buf = graphics
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(&indices_text),
            label: Some("i_text_buf"),
            usage: wgpu::BufferUsage::INDEX,
        });
    

    // render!
    let frame = graphics.swap_chain.get_current_frame()?.output;
    let mut command_buf = graphics
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("command_buf") });
    {
        let mut pass = command_buf.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0125,
                        b: 0.05,
                        a: 1.0,
                    }),
                    store: true,
                },
                resolve_target: None,
                view: &frame.view,
            }],
            depth_stencil_attachment: None,
        });
        // draw objects
        pass.set_pipeline(&graphics.pipeline);
        pass.set_vertex_buffer(0, v_buf.slice(..));
        pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, &graphics.mat_buffer_bind_group, &[]);
        pass.set_bind_group(1, &graphics.text_texture_bind_group, &[]); // ignored by shader
        pass.draw_indexed(0..indices.len() as _, 0, 0..1);

        // draw text
        pass.set_pipeline(&graphics.text_pipeline);
        pass.set_vertex_buffer(0, v_text_buf.slice(..));
        pass.set_index_buffer(i_text_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, &graphics.mat_buffer_bind_group, &[]);
        pass.set_bind_group(1, &graphics.text_texture_bind_group, &[]);
        pass.draw_indexed(0..indices_text.len() as _, 0, 0..1);
    }
    graphics.queue.submit(std::iter::once(command_buf.finish()));

    Ok(())
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (mut window, events) = glfw
        .create_window(800, 600, "tet.rs", glfw::WindowMode::Windowed)
        .expect("Failed to create window.");

    window.set_key_polling(true);
    window.set_size_polling(true);

    let mut state = GameState::default();
    let mut graphics = futures::executor::block_on(GraphicsState::new(&window));
    let mut last_frame = std::time::Instant::now();

    while !window.should_close() {
        // timing
        let frame = std::time::Instant::now();
        let dt = frame - last_frame;
        last_frame = frame;

        // update
        update(&window, &mut state, dt);

        // render
        match render(&state, &graphics) {
            Err(wgpu::SwapChainError::OutOfMemory) => window.set_should_close(true),
            Err(wgpu::SwapChainError::Lost) | Err(wgpu::SwapChainError::Outdated) => {
                graphics.resize(graphics.sc_desc.width, graphics.sc_desc.height)
            }
            _ => (),
        };

        // events
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true)
                }
                glfw::WindowEvent::Size(width, height) => {
                    graphics.resize(width as u32, height as u32);
                }
                _ => (),
            }
        }
    }
}
