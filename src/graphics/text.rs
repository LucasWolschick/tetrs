use super::Vertex;

const TEXT_IMAGE_COLUMNS: i32 = 16;
const TEXT_IMAGE_ROWS: i32 = 8;
const TEXT_CHARACTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789!?@#$%\"'&()*+,-./:;<>=[]{}|\\";

pub fn render_text(text: &str, x: f32, y: f32, size: f32, base_idx: usize, color: [f32; 3]) -> (Vec<Vertex>, Vec<u16>) {
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
                color,
                tex_coords: [char_x, char_y],
            },
            Vertex {
                position: [x + size + i as f32 * size, y, 0.0],
                color,
                tex_coords: [char_x + tile_size_x, char_y],
            },
            Vertex {
                position: [x + i as f32 * size, y + size / 2.0, 0.0],
                color,
                tex_coords: [char_x, char_y + tile_size_y],
            },
            Vertex {
                position: [x + size + i as f32 * size, y + size / 2.0, 0.0],
                color,
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