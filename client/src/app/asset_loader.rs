use client::screens::PlayingState;
use macroquad::prelude::*;

use super::obj_loader::load_obj_mesh_with_materials;

pub async fn load_asset_entry(key: String, path: String, playing_state: &mut PlayingState) {
    let is_model = path.to_lowercase().ends_with(".obj");

    if is_model {
        match load_obj_mesh_with_materials(&path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                playing_state.models.insert(key.clone(), mesh);
                println!(
                    "Loaded model: {} ({} verts) from {}",
                    key, vertex_count, path
                );
            }
            Err(e) => {
                println!("Failed to load model {} from {}: {:?}", key, path, e);
            }
        }
    } else {
        match load_texture(&path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key.clone(), tex);
                println!("Loaded texture: {} from {}", key, path);
            }
            Err(e) => {
                println!("Failed to load texture {} from {}: {:?}", key, path, e);
            }
        }
    }
}
