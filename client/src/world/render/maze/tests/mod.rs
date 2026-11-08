mod fixture;
mod oracle;
mod reference;
mod stats;

use std::collections::HashSet;

use macroquad::prelude::*;

use crate::types::maze::{classify_cells, CellKind, MazeChange, MazeSnapshot, PropKind};
use crate::world::render::props::PropIndex;

use super::cache::{bake_prop_group, chunks_touched, prop_groups};
use super::cells::{bake_chunk, chunk_grid, chunk_of};
use super::cull::{DistanceCull, Frustum};
use super::globals::bake_globals;
use super::lighting::Lighting;
use super::mesh::{MeshBuilder, MeshData};
use fixture::test_maze;
use oracle::{assert_floor_plane_covered, face_hidden};
use reference::{draw_cube_faces, old_cell_cubes, old_global_cubes, old_lights, Cube, Face};

/// The quads of `meshes` with their cull centres, checking each quad is
/// stored as `draw_quad` stores it.
fn quads(meshes: &[MeshData]) -> Vec<(Vec2, Face)> {
    let mut out = Vec::new();
    for m in meshes {
        assert_eq!(m.vertices.len() % 4, 0);
        assert_eq!(m.indices.len(), m.vertices.len() / 4 * 6);
        for (q, v) in m.vertices.chunks(4).enumerate() {
            let base = (q * 4) as u16;
            assert_eq!(&m.indices[q * 6..q * 6 + 6], &[0, 1, 2, 0, 2, 3].map(|i| base + i));
            assert!(v.iter().all(|x| x.cull == v[0].cull && x.color == v[0].color));
            let face = Face { corners: [v[0].pos, v[1].pos, v[2].pos, v[3].pos], color: v[0].color };
            out.push((Vec2::from_array(v[0].cull), face));
        }
    }
    out
}

fn all_faces(cubes: &[Cube]) -> Vec<Face> {
    cubes.iter().flat_map(draw_cube_faces).collect()
}

fn cell_of(m: &crate::types::maze::Maze, cull: Vec2) -> (usize, usize) {
    ((cull.x / m.cell_size).floor() as usize, (cull.y / m.cell_size).floor() as usize)
}

#[test]
fn cube_faces_match_draw_cube() {
    let cubes = [
        Cube { pos: vec3(1.3, 0.7, -2.9), size: vec3(0.33, 1.7, 0.05), color: Color::new(0.2, 0.5, 0.9, 1.0) },
        Cube { pos: vec3(37.8, -0.025, 12.6), size: vec3(1.8, 0.05, 1.8), color: Color::new(0.98, 0.94, 0.70, 0.25) },
    ];
    for cube in cubes {
        let mut b = MeshBuilder::new(true);
        b.cube(cube.pos, cube.size, cube.color);
        let got: Vec<Face> = quads(&b.finish()).into_iter().map(|(_, f)| f).collect();
        assert_eq!(got, draw_cube_faces(&cube).to_vec());
    }
}

/// Every baked cell face is one the old path drew, bit for bit and in the
/// same order within its cell; every face left out could never be seen.
#[test]
fn chunks_bake_the_visible_faces_of_the_old_path() {
    let m = test_maze();
    let destroyed = HashSet::new();
    let lights = old_lights(&m, &destroyed);
    let lighting = Lighting::new(&m, &destroyed);
    let (chunks_x, chunks_y) = chunk_grid(&m);
    let mut seen_cells = Vec::new();
    let mut kept = 0;
    let mut dropped = 0;
    for c in 0..chunks_x * chunks_y {
        let baked = quads(&bake_chunk(&m, &lighting, c % chunks_x, c / chunks_x));
        let mut cells: Vec<(usize, usize)> = baked.iter().map(|(cull, _)| cell_of(&m, *cull)).collect();
        cells.dedup();
        assert!(cells.iter().all(|&(x, y)| chunk_of(x, y, chunks_x) == c), "cell outside its chunk");
        for (x, y) in cells {
            seen_cells.push((x, y));
            let old = old_cell_cubes(&m, &lights, x, y);
            let mut ours = baked.iter().filter(|(cull, _)| cell_of(&m, *cull) == (x, y)).map(|(_, f)| *f).peekable();
            for (i, face) in all_faces(&old).into_iter().enumerate() {
                if ours.peek() == Some(&face) {
                    ours.next();
                    kept += 1;
                } else {
                    assert!(face_hidden(&m, &old[i / 6], i % 6), "cell ({x}, {y}) cube {} face {} dropped but visible", i / 6, i % 6);
                    dropped += 1;
                }
            }
            assert!(ours.next().is_none(), "cell ({x}, {y}) has a face the old path never drew");
        }
    }
    // Every cell is baked once, row by row within each chunk.
    seen_cells.sort();
    let mut expected: Vec<(usize, usize)> = (0..m.width as usize).flat_map(|x| (0..m.height as usize).map(move |y| (x, y))).collect();
    expected.sort();
    assert_eq!(seen_cells, expected);
    assert!(kept > 0 && dropped * 4 > kept, "kept {kept}, dropped {dropped}");
    assert_floor_plane_covered(&m, &lighting);
}

#[test]
fn globals_match_the_old_path_exactly() {
    let m = test_maze();
    for destroyed in [HashSet::new(), HashSet::from([(9u8, 8u8)])] {
        let lights = old_lights(&m, &destroyed);
        let baked: Vec<Face> = quads(&bake_globals(&m, &Lighting::new(&m, &destroyed), &destroyed))
            .into_iter()
            .map(|(_, f)| f)
            .collect();
        assert_eq!(baked, all_faces(&old_global_cubes(&m, &lights, &destroyed)));
    }
}

#[test]
fn breaking_a_wall_rebakes_exactly_the_chunks_it_touches() {
    let mut m = test_maze();
    let destroyed = HashSet::new();
    let lighting = Lighting::new(&m, &destroyed);
    let (chunks_x, chunks_y) = chunk_grid(&m);
    let bake_all = |m: &crate::types::maze::Maze| -> Vec<Vec<(Vec2, Face)>> {
        (0..chunks_x * chunks_y).map(|c| quads(&bake_chunk(m, &lighting, c % chunks_x, c / chunks_x))).collect()
    };
    let before = bake_all(&m);
    let mut snapshot = MazeSnapshot::capture(&m);

    // (8, 5) is the first cell of chunk 1; its neighbour (7, 5) is in chunk 0.
    assert!(m.damage_wall(8, 5, 1000.0));
    let MazeChange::Cells(changed) = snapshot.sync(&m) else { panic!("expected a cell change") };
    assert_eq!(changed, vec![(8, 5)]);
    let touched = chunks_touched(&m, &changed);
    assert_eq!(touched.len(), 2);
    assert!(touched.contains(&chunk_of(8, 5, chunks_x)) && touched.contains(&chunk_of(7, 5, chunks_x)));

    let after = bake_all(&m);
    for c in 0..chunks_x * chunks_y {
        if touched.contains(&c) {
            assert_ne!(before[c], after[c], "chunk {c} should change");
        } else {
            assert_eq!(before[c], after[c], "chunk {c} changed but was not rebaked");
        }
    }
    assert_eq!(snapshot.sync(&m), MazeChange::Unchanged);
}

#[test]
fn snapshot_tells_layout_changes_from_wall_breaks() {
    let mut m = test_maze();
    let mut snapshot = MazeSnapshot::capture(&m);
    assert_eq!(snapshot.sync(&m), MazeChange::Unchanged);
    m.force_break_wall(4, 3);
    assert_eq!(snapshot.sync(&m), MazeChange::Cells(vec![(4, 3)]));
    m.props.pop();
    assert_eq!(snapshot.sync(&m), MazeChange::Layout);
    m.ceiling_lights[0].variant = 2;
    assert_eq!(snapshot.sync(&m), MazeChange::Layout);
    assert_eq!(snapshot.sync(&m), MazeChange::Unchanged);
}

#[test]
fn shooting_a_light_recolours_cells_far_from_it() {
    let m = test_maze();
    let lit = Lighting::new(&m, &HashSet::new());
    let dark = Lighting::new(&m, &HashSet::from([(16u8, 3u8)]));
    // Chunk 0 is nowhere near the light at (16, 3), yet its colours change:
    // a light reaches every cube, which is why the whole maze is rebaked.
    assert_ne!(quads(&bake_chunk(&m, &lit, 0, 0)), quads(&bake_chunk(&m, &dark, 0, 0)));
}

#[test]
fn prop_groups_keep_list_order_and_geometry() {
    let m = test_maze();
    let lighting = Lighting::new(&m, &HashSet::new());
    let index = PropIndex::new(&m);
    let groups = prop_groups(&m);
    assert_eq!(groups.first().map(|g| g.start), Some(0));
    assert!(groups.windows(2).all(|w| w[0].end == w[1].start));
    assert_eq!(groups.last().map(|g| g.end), Some(m.props.len()));
    let (chunks_x, _) = chunk_grid(&m);
    for g in &groups {
        let chunk = |i: usize| chunk_of(m.props[i].x as usize, m.props[i].y as usize, chunks_x);
        assert!(g.clone().all(|i| chunk(i) == chunk(g.start)));
    }
    let grouped: Vec<_> = groups.into_iter().flat_map(|g| quads(&bake_prop_group(&m, &index, &lighting, g))).collect();
    assert_eq!(grouped, quads(&bake_prop_group(&m, &index, &lighting, 0..m.props.len())));
    // Each prop's geometry carries its own cell as cull centre.
    let centres: HashSet<(usize, usize)> = grouped.iter().map(|(c, _)| cell_of(&m, *c)).collect();
    let cells: HashSet<(usize, usize)> = m.props.iter().map(|p| (p.x as usize, p.y as usize)).collect();
    assert_eq!(centres, cells);
}

#[test]
fn prop_index_matches_scanning_every_prop() {
    let m = test_maze();
    let index = PropIndex::new(&m);
    let scan = |px: i32, py: i32, kind: fn(PropKind) -> bool| {
        [(0, -1), (0, 1), (-1, 0), (1, 0)].map(|(dx, dy): (i32, i32)| {
            let (nx, ny) = (px + dx, py + dy);
            nx >= 0 && ny >= 0 && m.props.iter().any(|p| p.x as i32 == nx && p.y as i32 == ny && kind(p.kind))
        })
    };
    for p in &m.props {
        let (px, py) = (p.x as i32, p.y as i32);
        assert_eq!(index.chairs_around(p), scan(px, py, |k| matches!(k, PropKind::OfficeChair)));
        assert_eq!(index.monitors_around(p), scan(px, py, |k| matches!(k, PropKind::Monitor)));
    }
    assert_eq!(index.chairs_around(&m.props[1]), [false, true, false, false]);
}

#[test]
fn minimap_cell_kinds_match_the_old_lookup() {
    let mut m = test_maze();
    m.force_break_wall(7, 5);
    let kinds = classify_cells(&m);
    let mut room = vec![false; kinds.len()];
    for r in &m.rooms {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                room[y as usize * m.width as usize + x as usize] = true;
            }
        }
    }
    for y in 0..m.height {
        for x in 0..m.width {
            let i = m.idx(x as usize, y as usize);
            let expected = match (m.is_open(x as i32, y as i32), room[i], m.is_breakable(x, y)) {
                (true, true, _) => CellKind::RoomFloor,
                (true, false, _) => CellKind::Floor,
                (false, _, true) => CellKind::BreakableWall,
                (false, _, false) => CellKind::Wall,
            };
            assert_eq!(kinds[i], expected, "cell ({x}, {y})");
        }
    }
    assert_eq!(kinds[m.idx(7, 5)], CellKind::Floor);
    assert_eq!(kinds[m.idx(8, 5)], CellKind::BreakableWall);
}

#[test]
fn frustum_only_drops_boxes_wholly_outside() {
    let view = Mat4::look_at_rh(vec3(0.0, 1.6, 0.0), vec3(0.0, 1.6, -1.0), Vec3::Y);
    let proj = Mat4::perspective_rh_gl(70f32.to_radians(), 16.0 / 9.0, 0.01, 100.0);
    let f = Frustum::new(proj * view);
    assert!(f.may_see(vec3(-1.0, 0.0, -6.0), vec3(1.0, 2.0, -5.0)), "in front");
    assert!(!f.may_see(vec3(-1.0, 0.0, 5.0), vec3(1.0, 2.0, 6.0)), "behind");
    assert!(f.may_see(vec3(-1.0, 0.0, -1.0), vec3(1.0, 2.0, 1.0)), "around the camera");
    assert!(f.may_see(vec3(-0.2, 0.0, 0.1), vec3(0.2, 2.0, 0.2)), "just behind, within the margin");
    assert!(!f.may_see(vec3(-1.0, 0.0, -150.0), vec3(1.0, 2.0, -140.0)), "past the far plane");
}

/// The old path drew every prop whatever the distance, so at a low render
/// distance a prop stays visible past its culled floor.
#[test]
fn far_props_stay_drawn_while_their_floor_is_culled() {
    let m = test_maze();
    let lighting = Lighting::new(&m, &HashSet::new());
    let foosball = m.props.iter().position(|p| matches!(p.kind, PropKind::Foosball)).unwrap();
    let (fx, fy) = (m.props[foosball].x as usize, m.props[foosball].y as usize);
    let viewer = m.cell_to_world(1, 1);
    let dist = DistanceCull { viewer: vec2(viewer.x, viewer.z), max_dist_sq: (3.0 * m.cell_size).powi(2) };
    let far = m.cell_to_world(fx as i32, fy as i32);
    assert!(!dist.keeps(vec2(far.x, far.z)), "fixture: the foosball must be out of range");

    // Its floor: the chunk is skipped and the cell culled per vertex.
    let (chunks_x, _) = chunk_grid(&m);
    let c = chunk_of(fx, fy, chunks_x);
    for mesh in bake_chunk(&m, &lighting, c % chunks_x, c / chunks_x) {
        assert!(!dist.may_keep(&mesh.bounds));
        let floor = mesh.items.iter().find(|i| i.cull == vec2(far.x, far.z)).unwrap();
        assert!(!dist.keeps_in(&mesh.bounds, floor.cull));
        assert_eq!(dist.shader_limit(&mesh.bounds), dist.max_dist_sq);
    }

    // The prop: kept by the pre-test, the CPU fallback and the shader.
    let index = PropIndex::new(&m);
    let group = prop_groups(&m).into_iter().find(|g| g.contains(&foosball)).unwrap();
    let meshes = bake_prop_group(&m, &index, &lighting, group);
    assert!(!meshes.is_empty());
    for mesh in &meshes {
        assert!(dist.may_keep(&mesh.bounds));
        assert!(mesh.items.iter().all(|i| dist.keeps_in(&mesh.bounds, i.cull)));
        assert_eq!(dist.shader_limit(&mesh.bounds), f32::MAX);
    }
}

#[test]
fn distance_pretest_never_drops_a_kept_cell() {
    let m = test_maze();
    let lighting = Lighting::new(&m, &HashSet::new());
    let (chunks_x, chunks_y) = chunk_grid(&m);
    let meshes: Vec<MeshData> = (0..chunks_x * chunks_y).flat_map(|c| bake_chunk(&m, &lighting, c % chunks_x, c / chunks_x)).collect();
    for (vx, vz, r) in [(3.0, 3.0, 9.0), (20.0, 11.0, 5.0), (-30.0, 4.0, 31.0), (18.9, 9.0, 18.0 + 2.7)] {
        let dist = DistanceCull { viewer: vec2(vx, vz), max_dist_sq: r * r };
        for mesh in &meshes {
            if mesh.items.iter().any(|i| dist.keeps(i.cull)) {
                assert!(dist.may_keep(&mesh.bounds));
            }
        }
    }
}
