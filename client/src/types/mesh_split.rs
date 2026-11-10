use std::collections::HashMap;

use macroquad::models::{Mesh, Vertex};

/*
    macroquad draws at most 10,000 vertices and 5,000 indices per draw call and silently cuts the rest
    (it only logs "geometry() exceeded max drawcall size, clamping"), so big meshes lost triangles,
    the hands' arms drew about 12% and several guns had chunks missing
    this cuts a mesh into pieces that each fit, at load, so every triangle gets drawn
    raising macroquad's limit instead would reserve that much memory for every draw call slot
*/

// whole triangles only, so a multiple of 3 just under 5,000
pub const MAX_DRAW_INDICES: usize = 4998;
pub const MAX_DRAW_VERTICES: usize = 10_000;

// one piece: its triangles (indices into `old_ids`), and which vertices of the original it uses
// old_ids are sorted, so anything that keeps vertices in groups (skinned first, then rigid) stays grouped
pub struct Chunk {
    pub indices: Vec<u16>,
    pub old_ids: Vec<u16>,
}

// cut a triangle list into pieces under the draw limits, one piece if it already fits
pub fn chunk_triangles(indices: &[u16], vertex_count: usize) -> Vec<Chunk> {
    if indices.len() <= MAX_DRAW_INDICES && vertex_count <= MAX_DRAW_VERTICES {
        let old_ids = (0..vertex_count as u16).collect();
        return vec![Chunk { indices: indices.to_vec(), old_ids }];
    }
    let mut chunks = Vec::new();
    let mut tris: Vec<[u16; 3]> = Vec::new();
    let mut used: HashMap<u16, ()> = HashMap::new();
    for tri in indices.chunks_exact(3) {
        let new_verts = tri.iter().filter(|v| !used.contains_key(v)).count();
        let full = (tris.len() + 1) * 3 > MAX_DRAW_INDICES || used.len() + new_verts > MAX_DRAW_VERTICES;
        if full && !tris.is_empty() {
            chunks.push(finish_chunk(&tris));
            tris.clear();
            used.clear();
        }
        for v in tri {
            used.insert(*v, ());
        }
        tris.push([tri[0], tri[1], tri[2]]);
    }
    if !tris.is_empty() {
        chunks.push(finish_chunk(&tris));
    }
    chunks
}

fn finish_chunk(tris: &[[u16; 3]]) -> Chunk {
    let mut old_ids: Vec<u16> = tris.iter().flatten().copied().collect();
    old_ids.sort_unstable();
    old_ids.dedup();
    let new_of: HashMap<u16, u16> = old_ids.iter().enumerate().map(|(n, &o)| (o, n as u16)).collect();
    let indices = tris.iter().flatten().map(|o| new_of[o]).collect();
    Chunk { indices, old_ids }
}

// split a plain mesh (static models, guns), same texture on every piece
pub fn split_mesh(mesh: Mesh) -> Vec<Mesh> {
    let chunks = chunk_triangles(&mesh.indices, mesh.vertices.len());
    if chunks.len() == 1 {
        return vec![mesh];
    }
    chunks
        .into_iter()
        .map(|c| Mesh {
            vertices: c.old_ids.iter().map(|&o| mesh.vertices[o as usize]).collect::<Vec<Vertex>>(),
            indices: c.indices,
            texture: mesh.texture.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // a strip of n quads = 2n triangles, 2n+2 vertices
    fn strip(quads: u16) -> Vec<u16> {
        (0..quads)
            .flat_map(|q| {
                let a = q * 2;
                [a, a + 1, a + 2, a + 1, a + 3, a + 2]
            })
            .collect()
    }

    #[test]
    fn small_meshes_stay_whole() {
        let idx = strip(10);
        let chunks = chunk_triangles(&idx, 22);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].indices, idx);
    }

    #[test]
    fn big_meshes_split_under_the_limit_and_keep_every_triangle() {
        let idx = strip(4000); // 24,000 indices, way over 5,000
        let chunks = chunk_triangles(&idx, 8002);
        assert!(chunks.len() >= 5);
        let mut total = 0;
        for c in &chunks {
            assert!(c.indices.len() <= MAX_DRAW_INDICES && c.indices.len() % 3 == 0);
            assert!(c.old_ids.len() <= MAX_DRAW_VERTICES);
            assert!(c.old_ids.windows(2).all(|w| w[0] < w[1]), "old ids sorted");
            total += c.indices.len();
        }
        assert_eq!(total, idx.len());
        // the first triangle comes back as the same original vertices
        let c = &chunks[0];
        let first: Vec<u16> = c.indices[..3].iter().map(|&n| c.old_ids[n as usize]).collect();
        assert_eq!(first, idx[..3]);
    }
}
