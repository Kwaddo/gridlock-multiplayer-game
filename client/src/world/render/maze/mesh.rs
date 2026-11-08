//! CPU-side baked geometry: cubes emitted face by face exactly as
//! macroquad's `draw_cube` would, so the GPU draws the same vertices.

use macroquad::prelude::*;

use super::constants::MAX_MESH_VERTICES;

/// Faces of a cube, in `draw_cube`'s order.
pub(in crate::world::render) mod face {
    pub const FRONT: u8 = 1; // +z
    pub const BACK: u8 = 2; // -z
    pub const TOP: u8 = 4; // +y
    pub const BOTTOM: u8 = 8; // -y
    pub const RIGHT: u8 = 16; // +x
    pub const LEFT: u8 = 32; // -x
    pub const ALL: u8 = 63;
}

/// One baked vertex: position, colour as `draw_cube` stores it, and the
/// centre (x, z) of the cell the vertex belongs to, which decides whether it
/// is beyond the render distance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::world::render) struct BakedVertex {
    pub pos: [f32; 3],
    pub color: [u8; 4],
    pub cull: [f32; 2],
}

/// A run of geometry sharing one cull centre (a cell, a prop, a fixture).
#[derive(Clone, Copy, Debug)]
pub(in crate::world::render) struct Item {
    pub first_vertex: usize,
    pub vertex_count: usize,
    pub first_index: usize,
    pub index_count: usize,
    pub cull: Vec2,
}

/// What culling needs to know about a mesh.
#[derive(Clone, Copy, Debug)]
pub(in crate::world::render) struct Bounds {
    /// Bounds of every vertex.
    pub min: Vec3,
    pub max: Vec3,
    /// Bounds of every item's cull centre.
    pub cull_min: Vec2,
    pub cull_max: Vec2,
    /// False: drawn whatever the distance (roof, pipes, fixtures, props).
    pub cullable: bool,
}

/// Geometry for one draw call.
pub(in crate::world::render) struct MeshData {
    pub vertices: Vec<BakedVertex>,
    pub indices: Vec<u16>,
    pub items: Vec<Item>,
    pub bounds: Bounds,
}

impl MeshData {
    fn new(cullable: bool) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            items: Vec::new(),
            bounds: Bounds {
                min: Vec3::splat(f32::INFINITY),
                max: Vec3::splat(f32::NEG_INFINITY),
                cull_min: Vec2::splat(f32::INFINITY),
                cull_max: Vec2::splat(f32::NEG_INFINITY),
                cullable,
            },
        }
    }

    fn start_item(&mut self, cull: Vec2) {
        self.items.push(Item {
            first_vertex: self.vertices.len(),
            vertex_count: 0,
            first_index: self.indices.len(),
            index_count: 0,
            cull,
        });
        self.bounds.cull_min = self.bounds.cull_min.min(cull);
        self.bounds.cull_max = self.bounds.cull_max.max(cull);
    }
}

pub(in crate::world::render) struct MeshBuilder {
    cullable: bool,
    cull: Vec2,
    meshes: Vec<MeshData>,
}

impl MeshBuilder {
    pub fn new(cullable: bool) -> Self {
        Self { cullable, cull: Vec2::ZERO, meshes: vec![MeshData::new(cullable)] }
    }

    /// Following cubes belong to the cell (or prop) centred at `cull`.
    pub fn begin_item(&mut self, cull: Vec2) {
        self.cull = cull;
        let mesh = self.meshes.last_mut().expect("builder always has a mesh");
        if mesh.items.last().is_some_and(|i| i.vertex_count == 0) {
            mesh.items.pop();
        }
        mesh.start_item(cull);
    }

    /// Every face of a cube, like `draw_cube(position, size, None, color)`.
    pub fn cube(&mut self, position: Vec3, size: Vec3, color: Color) {
        self.cube_faces(position, size, color, face::ALL);
    }

    /// The `faces` of a cube, each with the vertices `draw_cube` gives it.
    pub fn cube_faces(&mut self, position: Vec3, size: Vec3, color: Color, faces: u8) {
        let color: [u8; 4] = color.into();
        let (x, y, z) = (position.x, position.y, position.z);
        let (width, height, length) = (size.x, size.y, size.z);
        let (x0, x1) = (x - width / 2., x + width / 2.);
        let (y0, y1) = (y - height / 2., y + height / 2.);
        let (z0, z1) = (z - length / 2., z + length / 2.);
        if faces & face::FRONT != 0 {
            self.quad([vec3(x0, y0, z1), vec3(x1, y0, z1), vec3(x1, y1, z1), vec3(x0, y1, z1)], color);
        }
        if faces & face::BACK != 0 {
            self.quad([vec3(x0, y0, z0), vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x0, y1, z0)], color);
        }
        if faces & face::TOP != 0 {
            self.quad([vec3(x0, y1, z0), vec3(x0, y1, z1), vec3(x1, y1, z1), vec3(x1, y1, z0)], color);
        }
        if faces & face::BOTTOM != 0 {
            self.quad([vec3(x0, y0, z0), vec3(x0, y0, z1), vec3(x1, y0, z1), vec3(x1, y0, z0)], color);
        }
        if faces & face::RIGHT != 0 {
            self.quad([vec3(x1, y0, z0), vec3(x1, y1, z0), vec3(x1, y1, z1), vec3(x1, y0, z1)], color);
        }
        if faces & face::LEFT != 0 {
            self.quad([vec3(x0, y0, z0), vec3(x0, y1, z0), vec3(x0, y1, z1), vec3(x0, y0, z1)], color);
        }
    }

    fn quad(&mut self, corners: [Vec3; 4], color: [u8; 4]) {
        if self.meshes.last().is_some_and(|m| m.vertices.len() + 4 > MAX_MESH_VERTICES) {
            self.meshes.push(MeshData::new(self.cullable));
        }
        let cull = self.cull;
        let mesh = self.meshes.last_mut().expect("builder always has a mesh");
        if mesh.items.is_empty() {
            mesh.start_item(cull);
        }
        let base = mesh.vertices.len() as u16;
        for p in corners {
            mesh.bounds.min = mesh.bounds.min.min(p);
            mesh.bounds.max = mesh.bounds.max.max(p);
            mesh.vertices.push(BakedVertex { pos: p.to_array(), color, cull: cull.to_array() });
        }
        // `draw_quad`'s triangles.
        mesh.indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        let item = mesh.items.last_mut().expect("item started above");
        item.vertex_count += 4;
        item.index_count += 6;
    }

    /// The baked meshes, empty ones dropped.
    pub fn finish(self) -> Vec<MeshData> {
        self.meshes
            .into_iter()
            .filter(|m| !m.vertices.is_empty())
            .map(|mut m| {
                m.items.retain(|i| i.vertex_count > 0);
                m
            })
            .collect()
    }
}
