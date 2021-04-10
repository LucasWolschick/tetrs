use cgmath::prelude::*;
use cgmath::Vector2;

use super::Vertex;

pub fn render_lines_pairs(positions: &[Vector2<f32>], mut thickness: f32, index_offset: usize) -> (Vec<Vertex>, Vec<u16>) {
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