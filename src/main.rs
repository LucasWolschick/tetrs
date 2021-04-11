use glfw::{Action, Key};
use rand::prelude::*;
use wgpu::util::DeviceExt;

use std::array;

use wgpu_practice as lib;
use lib::{game::GameState, graphics::Vertex};

const FIELD_WIDTH: u32 = 10;
const FIELD_HEIGHT: u32 = 20;
const FRAME_TIME: f32 = 0.05;
const ACTIVE_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const INACTIVE_COLOR: [f32; 3] = [0.5, 0.5, 0.5];

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

fn was_pressed(input: KeyState, ticker: u64) -> bool {
    match input {
        KeyState::Pressed => true,
        KeyState::Holding if ticker % 2 == 0 => true,
        _ => false,
    }
}

struct TetrisMenu {
    // Current menu selection
    selection: u8,

    // Previous frame player input
    last_input: PlayerInput,

    /// Time accumulator
    accum: f32,

    /// Current frame number
    ticker: u64,
}

impl Default for TetrisMenu {
    fn default() -> Self {
        TetrisMenu {
            selection: 0,
            last_input: PlayerInput::all_pressed(),
            accum: 0.0,
            ticker: 0,
        }
    }
}

impl GameState for TetrisMenu {
    fn update(&mut self, window: &glfw::Window, dt: std::time::Duration) -> lib::game::StateChange {
        self.accum += dt.as_secs_f32();

        while self.accum >= FRAME_TIME {
            self.accum -= FRAME_TIME;
            self.ticker += 1;

            let input = input(window, self.last_input);
            self.last_input = input;
            if input.rot_left == KeyState::Pressed || input.rot_right == KeyState::Pressed {
                // confirm choice.
                match self.selection {
                    0 => {
                        // load game
                        return lib::game::StateChange::Push(Box::new(TetrisMain::default()));
                    }
                    1 => {
                        // quit game
                        return lib::game::StateChange::Quit;
                    }
                    _ => unreachable!(),
                }
            } else if input.up == KeyState::Pressed {
                // move selection up
                if self.selection == 0 {
                    self.selection = 1;
                } else {
                    self.selection -= 1;
                }
            } else if input.down == KeyState::Pressed {
                // move selection down
                if self.selection == 1 {
                    self.selection = 0;
                } else {
                    self.selection += 1;
                }
            }
        }

        lib::game::StateChange::None
    }

    fn render(&self, graphics: &lib::graphics::GraphicsState) -> Result<(), wgpu::SwapChainError> {
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

        let (vt, it) = lib::graphics::text::render_text("Tet.rs", 0.0, 0.2, 1.0/6.0, vertices_text.len(), ACTIVE_COLOR);
        vertices_text.extend(vt);
        indices_text.extend(it);

        let (vt, it) = lib::graphics::text::render_text("Play", 0.25, 0.5, 0.5/4.0, vertices_text.len(), if self.selection == 0 { ACTIVE_COLOR } else { INACTIVE_COLOR });
        vertices_text.extend(vt);
        indices_text.extend(it);

        let (vt, it) = lib::graphics::text::render_text("Quit", 0.25, 0.7, 0.5/4.0, vertices_text.len(), if self.selection == 1 { ACTIVE_COLOR } else { INACTIVE_COLOR });
        vertices_text.extend(vt);
        indices_text.extend(it);

        // render selection tick on highlighted thingie
        let y_offset = match self.selection {
            0 => 0.5,
            1 => 0.7,
            _ => unreachable!(),
        };
        let tri_width = 0.5/4.0/2.0;
        let x_offset = 0.25 - tri_width * 1.5;
        let vertices_tri = vec![
            Vertex {
                position: [x_offset, y_offset, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [0.0, 0.0]
            },
            Vertex {
                position: [x_offset + tri_width, y_offset + tri_width/2.0, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [0.0, 0.0]
            },
            Vertex {
                position: [x_offset, y_offset + tri_width, 0.0],
                color: [1.0, 1.0, 1.0],
                tex_coords: [0.0, 0.0]
            }
        ];
        let indices_tri: Vec<u16> = vec![
            0, 2, 1
        ];

        // create buffers
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
        let v_tri_buf = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&vertices_tri),
                label: Some("v_text_buf"),
                usage: wgpu::BufferUsage::VERTEX,
            });
        let i_tri_buf = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: bytemuck::cast_slice(&indices_tri),
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

            // draw text
            pass.set_pipeline(&graphics.text_pipeline);
            pass.set_vertex_buffer(0, v_text_buf.slice(..));
            pass.set_index_buffer(i_text_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_bind_group(0, &graphics.mat_buffer_bind_group, &[]);
            pass.set_bind_group(1, &graphics.text_texture_bind_group, &[]);
            pass.draw_indexed(0..indices_text.len() as _, 0, 0..1);

            // draw triangle
            pass.set_pipeline(&graphics.pipeline);
            pass.set_vertex_buffer(0, v_tri_buf.slice(..));
            pass.set_index_buffer(i_tri_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_bind_group(0, &graphics.mat_buffer_bind_group, &[]);
            pass.set_bind_group(1, &graphics.text_texture_bind_group, &[]);
            pass.draw_indexed(0..indices_tri.len() as _, 0, 0..1);
        }
        graphics.queue.submit(std::iter::once(command_buf.finish()));

        Ok(())
    }
}

struct TetrisMain {
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

impl lib::game::GameState for TetrisMain {
    fn update(&mut self, window: &glfw::Window, dt: std::time::Duration) -> lib::game::StateChange {
        self.accum += dt.as_secs_f32();
        
        while self.accum > FRAME_TIME {
            self.ticker += 1;

            let input = input(&window, self.last_input);
            self.last_input = input;
            self.accum -= FRAME_TIME;

            if was_pressed(input.escape, self.ticker) {
                return lib::game::StateChange::Pop;
            }

            if self.active_piece.is_none() {
                self.active_piece = Some(self.next_pieces.remove(0));
                self
                    .next_pieces
                    .push(Piece::new(rand::thread_rng().gen_range(0..PIECES.len())));
            }

            let mut active_piece = self.active_piece.as_mut().unwrap();

            // tick fall counter
            self.fall_counter -= 1;
            let should_fall = self.fall_counter == 0 || was_pressed(input.down, self.ticker);

            // tick down fall accelerator counter
            self.fall_accel_counter -= 1;
            if self.fall_accel_counter == 0 {
                self.fall_ticks = u32::max(self.fall_ticks - 1, 1);
                self.fall_accel_counter = self.fall_accel_ticks;
            }

            // rotate brick if requested
            if input.rot_right == KeyState::Pressed {
                if !self.rotated {
                    self.rotated = true;
                    let mut test_piece = active_piece.to_owned();
                    test_piece.rot = (test_piece.rot + 1) % 4;
                    if piece_fits(&test_piece, &self.field) {
                        active_piece.rot = test_piece.rot;
                    }
                }
            } else if input.rot_left == KeyState::Pressed {
                if !self.rotated {
                    self.rotated = true;
                    let mut test_piece = active_piece.to_owned();
                    test_piece.rot = if test_piece.rot == 0 {
                        3
                    } else {
                        test_piece.rot - 1
                    };
                    if piece_fits(&test_piece, &self.field) {
                        active_piece.rot = test_piece.rot;
                    }
                }
            } else {
                self.rotated = false;
            }

            // move brick left and right if requested
            if was_pressed(input.right, self.ticker) {
                let mut test_piece = active_piece.to_owned();
                test_piece.x += 1;
                if piece_fits(&test_piece, &self.field) {
                    active_piece.x = test_piece.x;
                }
            } else if was_pressed(input.left, self.ticker) {
                let mut test_piece = active_piece.to_owned();
                test_piece.x -= 1;
                if piece_fits(&test_piece, &self.field) {
                    active_piece.x = test_piece.x;
                }
            }

            // make piece fall
            if should_fall {
                self.fall_counter = self.fall_ticks;

                // verify if we can fall
                let mut test_piece = active_piece.to_owned();
                test_piece.y += 1;
                if piece_fits(&test_piece, &self.field) {
                    // fall
                    active_piece.y += 1;
                } else {
                    // add to board
                    add_piece(active_piece, &mut self.field);

                    // check if any lines are deletable
                    let mut deletable = Vec::new();
                    'outer_loop: for y in active_piece.y..active_piece.y + 4 {
                        if y < 0 {
                            // there's nothing here; continue
                            continue;
                        }
                        if i32::from(y) >= FIELD_HEIGHT as i32 {
                            // we've already passed the whole board; stop
                            break;
                        }
                        for x in 0..FIELD_WIDTH {
                            let tile = self.field[x as usize + y as usize * FIELD_WIDTH as usize];
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
                        self.score += match deletable.len() {
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
                                        self.field[x as usize + y as usize * FIELD_WIDTH as usize] =
                                            Cell::Empty;
                                    } else {
                                        // fill it with the contents of the line above
                                        self.field[x as usize + y as usize * FIELD_WIDTH as usize] = self
                                            .field
                                            [x as usize + (y - 1) as usize * FIELD_WIDTH as usize];
                                    }
                                }
                            }
                        }
                    }

                    // invalidate piece
                    self.active_piece = None;
                }
            }
        }

        lib::game::StateChange::None
    }

    fn render(&self, graphics: &lib::graphics::GraphicsState) -> Result<(), wgpu::SwapChainError> {
        const LINE_THICKNESS: f32 = 0.01;

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

        let mut vec_pairs = Vec::with_capacity((((FIELD_HEIGHT-1) + (FIELD_WIDTH-1))*2) as usize);
        for y in 1..FIELD_HEIGHT {
            vec_pairs.push(cgmath::Vector2::<f32>::new(0.0, y as f32 / FIELD_HEIGHT as f32));
            vec_pairs.push(cgmath::Vector2::<f32>::new(1.0, y as f32 / FIELD_HEIGHT as f32));
        }
        let (l_vtx, l_indx) = lib::graphics::lines::render_lines_pairs(&vec_pairs, LINE_THICKNESS / 2.0, vertices.len());
        vertices.extend(l_vtx);
        indices.extend(l_indx);
        vec_pairs.clear();

        for x in 1..FIELD_WIDTH {
            vec_pairs.push(cgmath::Vector2::<f32>::new(x as f32 / FIELD_WIDTH as f32, 0.0));
            vec_pairs.push(cgmath::Vector2::<f32>::new(x as f32 / FIELD_WIDTH as f32, 1.0));
        }
        let (l_vtx, l_indx) = lib::graphics::lines::render_lines_pairs(&vec_pairs, LINE_THICKNESS, vertices.len());
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
                if let Cell::Full(col) = self.field[(x + y * FIELD_WIDTH) as usize] {
                    add_cell(x, y, col);
                }
            }
        }

        // render active piece
        if let Some(piece) = self.active_piece {
            for y in 0..4 {
                for x in 0..4 {
                    if piece.filled_at(x, y) {
                        add_cell(
                            (i32::from(piece.x) + x as i32) as u32,
                            (i32::from(piece.y) + y as i32) as u32,
                            piece.color,
                        );
                    }
                }
            }
        }

        // render next pieces
        for (i, piece) in self.next_pieces.iter().enumerate() {
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
        
        let (vt, it) = lib::graphics::text::render_text(&format!("Score: {:06}", self.score), 1.1, 0.9, 0.05, vertices_text.len(), ACTIVE_COLOR);
        vertices_text.extend(vt);
        indices_text.extend(it);

        let level = 20 - self.fall_ticks + 1;

        let (vt, it) = lib::graphics::text::render_text(&format!("Level: {:2}", level), 1.1, 0.95, 0.05, vertices_text.len(), ACTIVE_COLOR);
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
}

impl Default for TetrisMain {
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
    up: KeyState,
    down: KeyState,
    left: KeyState,
    right: KeyState,
    rot_right: KeyState,
    rot_left: KeyState,
    escape: KeyState
}

impl PlayerInput {
    fn all_pressed() -> Self {
        Self {
            up: KeyState::Holding,
            down: KeyState::Holding,
            left: KeyState::Holding,
            right: KeyState::Holding,
            rot_right: KeyState::Holding,
            rot_left: KeyState::Holding,
            escape: KeyState::Holding,
        }
    }
}

fn input(window: &glfw::Window, last_input: PlayerInput) -> PlayerInput {
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
        up: map(window.get_key(Key::Up), last_input.up),
        down: map(window.get_key(Key::Down), last_input.down),
        left: map(window.get_key(Key::Left), last_input.left),
        right: map(window.get_key(Key::Right), last_input.right),
        rot_left: map(window.get_key(Key::X), last_input.rot_left),
        rot_right: map(window.get_key(Key::Z), last_input.rot_right),
        escape: map(window.get_key(Key::Escape), last_input.escape),
    }
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

    let (mut window, events) = glfw
        .create_window(800, 600, "tet.rs", glfw::WindowMode::Windowed)
        .expect("Failed to create window.");

    window.set_key_polling(true);
    window.set_size_polling(true);

    let mut states: Vec<Box<dyn GameState>> = vec![Box::new(TetrisMenu::default())];
    let mut graphics = futures::executor::block_on(lib::graphics::GraphicsState::new(&window));
    let mut last_frame = std::time::Instant::now();

    while !window.should_close() {
        let state = states.last_mut().unwrap();

        // timing
        let frame = std::time::Instant::now();
        let dt = frame - last_frame;
        last_frame = frame;

        // update
        let update_result = state.update(&window, dt);

        // render
        match state.render(&graphics) {
            Err(wgpu::SwapChainError::OutOfMemory) => window.set_should_close(true),
            Err(wgpu::SwapChainError::Lost) | Err(wgpu::SwapChainError::Outdated) => {
                graphics.resize(graphics.sc_desc.width, graphics.sc_desc.height)
            }
            _ => (),
        };

        match update_result {
            lib::game::StateChange::None => {} // do nothing
            lib::game::StateChange::Quit => {
                // quit the game
                window.set_should_close(true)
            }
            lib::game::StateChange::Push(state) => {
                // push a new state
                states.push(state);
            }
            lib::game::StateChange::Pop => {
                // pop state and quit if there are no more states
                if states.pop().is_none() {
                    window.set_should_close(true);
                }
            }
            lib::game::StateChange::Swap(state) => {
                // replace the current state by another one
                *states.last_mut().unwrap() = state;
            }
        }

        // events
        glfw.poll_events();

        #[allow(clippy::single_match)]
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Size(width, height) => {
                    graphics.resize(width as u32, height as u32);
                }
                _ => (),
            }
        }
    }
}
