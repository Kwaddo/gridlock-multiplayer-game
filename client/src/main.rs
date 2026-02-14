use macroquad::prelude::*;
use std::collections::HashMap;
use std::net::UdpSocket;

use client::core::{GameConfig, GameState};
use client::net::poll_updates;
use client::screens::dev_mode::{draw_dev_overlay, DevModeState};
use client::screens::{draw_pause_menu, LoginScreen, PlayingState};
use client::settings::SettingsScreen;
use client::types::maze::Maze;
use client::ui::fonts;
use client::weapons::crosshair_presets::CrosshairPresetManager;

fn resolve_sibling_path(base_path: &str, sibling_name: &str) -> String {
    if let Some((dir, _)) = base_path.rsplit_once('/') {
        format!("{}/{}", dir, sibling_name)
    } else {
        sibling_name.to_string()
    }
}

fn parse_obj_index(raw: &str, len: usize) -> Option<usize> {
    if raw.is_empty() {
        return None;
    }
    let idx = raw.parse::<i32>().ok()?;
    if idx > 0 {
        Some((idx - 1) as usize)
    } else if idx < 0 {
        let resolved = len as i32 + idx;
        (resolved >= 0).then_some(resolved as usize)
    } else {
        None
    }
}

fn parse_face_ref(token: &str, pos_len: usize, norm_len: usize) -> Option<(usize, Option<usize>)> {
    let mut parts = token.split('/');
    let v = parts.next().unwrap_or("");
    let _vt = parts.next();
    let vn = parts.next();

    let v_idx = parse_obj_index(v, pos_len)?;
    let n_idx = vn.and_then(|s| parse_obj_index(s, norm_len));
    Some((v_idx, n_idx))
}

fn parse_mtl_diffuse_colors(mtl_text: &str) -> HashMap<String, [u8; 4]> {
    fn boost_channel(v: f32) -> u8 {
        // Quaternius weapon materials use very dark Kd values; boost for readable in-game viewmodels.
        let boosted = (v.clamp(0.0, 1.0).powf(0.6) * 1.65).clamp(0.0, 1.0);
        (boosted * 255.0) as u8
    }

    let mut colors = HashMap::new();
    let mut current_material: Option<String> = None;

    for line in mtl_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("newmtl ") {
            current_material = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("Kd ") {
            if let Some(name) = &current_material {
                let comps: Vec<&str> = rest.split_whitespace().collect();
                if comps.len() >= 3 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        comps[0].parse::<f32>(),
                        comps[1].parse::<f32>(),
                        comps[2].parse::<f32>(),
                    ) {
                        colors.insert(
                            name.clone(),
                            [boost_channel(r), boost_channel(g), boost_channel(b), 255],
                        );
                    }
                }
            }
        }
    }

    colors
}

async fn load_obj_mesh_with_materials(obj_path: &str) -> Result<Mesh, String> {
    let obj_text = load_string(obj_path)
        .await
        .map_err(|e| format!("failed to read OBJ '{}': {:?}", obj_path, e))?;

    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut material_colors: HashMap<String, [u8; 4]> = HashMap::new();
    let mut current_color: [u8; 4] = [220, 220, 220, 255];

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    for line in obj_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("mtllib ") {
            let mtl_name = rest.trim();
            let candidates = [
                resolve_sibling_path(obj_path, mtl_name),
                mtl_name.to_string(),
            ];
            for candidate in &candidates {
                if let Ok(mtl_text) = load_string(candidate).await {
                    material_colors.extend(parse_mtl_diffuse_colors(&mtl_text));
                    break;
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("usemtl ") {
            let mat_name = rest.trim();
            if let Some(color) = material_colors.get(mat_name) {
                current_color = *color;
            }
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    positions.push(vec3(x, y, z));
                }
            }
            "vn" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    let n = vec3(x, y, z);
                    let n = if n.length_squared() > 0.0 {
                        n.normalize()
                    } else {
                        vec3(0.0, 1.0, 0.0)
                    };
                    normals.push(n);
                }
            }
            "f" if parts.len() >= 4 => {
                let refs: Vec<(usize, Option<usize>)> = parts[1..]
                    .iter()
                    .filter_map(|token| parse_face_ref(token, positions.len(), normals.len()))
                    .collect();

                if refs.len() < 3 {
                    continue;
                }

                for tri_i in 1..(refs.len() - 1) {
                    let tri = [refs[0], refs[tri_i], refs[tri_i + 1]];

                    let p0 = positions.get(tri[0].0).copied().unwrap_or(Vec3::ZERO);
                    let p1 = positions.get(tri[1].0).copied().unwrap_or(Vec3::ZERO);
                    let p2 = positions.get(tri[2].0).copied().unwrap_or(Vec3::ZERO);
                    let computed = (p1 - p0).cross(p2 - p0);
                    let face_normal = if computed.length_squared() > 0.0 {
                        computed.normalize()
                    } else {
                        vec3(0.0, 1.0, 0.0)
                    };

                    for (v_idx, n_idx_opt) in tri {
                        let pos = match positions.get(v_idx) {
                            Some(p) => *p,
                            None => continue,
                        };
                        let normal = n_idx_opt
                            .and_then(|n_idx| normals.get(n_idx).copied())
                            .unwrap_or(face_normal);

                        if vertices.len() >= u16::MAX as usize {
                            return Err(format!(
                                "OBJ '{}' has too many generated vertices for u16 index buffer",
                                obj_path
                            ));
                        }
                        let idx = vertices.len() as u16;
                        vertices.push(Vertex {
                            position: pos,
                            uv: Vec2::ZERO,
                            color: current_color,
                            normal: normal.extend(0.0),
                        });
                        indices.push(idx);
                    }
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(format!("OBJ '{}' produced empty mesh", obj_path));
    }

    // Normalize model space so all imported weapons share a sane pivot and scale.
    // This keeps viewmodels stable and prevents extreme local offsets from clipping.
    let mut min = vec3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = vec3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for v in &vertices {
        min.x = min.x.min(v.position.x);
        min.y = min.y.min(v.position.y);
        min.z = min.z.min(v.position.z);
        max.x = max.x.max(v.position.x);
        max.y = max.y.max(v.position.y);
        max.z = max.z.max(v.position.z);
    }

    let center = (min + max) * 0.5;
    let size = max - min;
    let max_extent = size.x.max(size.y).max(size.z).max(0.0001);
    for v in &mut vertices {
        v.position = (v.position - center) / max_extent;
    }

    Ok(Mesh {
        vertices,
        indices,
        texture: None,
    })
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Multiplayer FPS".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Set Nearest filtering globally so all textures (sprites, crosshairs) stay crisp
    set_default_filter_mode(FilterMode::Nearest);

    fonts::init_font().await;

    let mut state = GameState::Login;
    let mut login_screen = LoginScreen::new();
    let mut settings_screen = SettingsScreen::new();
    let mut playing_state = PlayingState::new().await;

    let mut socket: Option<UdpSocket> = None;
    let mut player_id: u32 = 0;
    let mut username = String::new();
    let mut server_addr = String::new();
    let mut config = GameConfig::new();
    let mut preset_manager = CrosshairPresetManager::new("");
    let mut maze: Option<Maze> = None;
    let mut is_fullscreen = true;

    // loading state tracking
    let mut loading_assets: Vec<(String, String)> = Vec::new();
    let mut loading_index: usize = 0;
    let mut loading_current: String = String::new();
    let mut loading_stable_frames: u8 = 0;
    let mut loading_started: bool = false;

    loop {
        if is_key_pressed(KeyCode::F11) {
            is_fullscreen = !is_fullscreen;
            set_fullscreen(is_fullscreen);
        }

        match state {
            GameState::Login => {
                if let Some((s, name, id)) = login_screen.update() {
                    socket = Some(s);
                    username = name;
                    player_id = id;
                    server_addr = login_screen.ip_input.clone();

                    // rebuild preset manager now that we know the username
                    preset_manager = CrosshairPresetManager::new(&username);

                    // reset loading state and transition to Loading
                    loading_assets.clear();
                    loading_index = 0;
                    loading_current.clear();
                    loading_stable_frames = 0;
                    loading_started = false;
                    state = GameState::Loading;
                }
                login_screen.draw();
            }

            GameState::Loading => {
                login_screen.update_background();

                // poll for network data (maze, assets)
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if let Some(entries) = assets {
                    if !entries.is_empty() {
                        loading_assets.extend(entries);
                        loading_stable_frames = 0;
                    }
                }

                if !loading_started {
                    loading_stable_frames += 1;
                    // start once we've collected asset packets and they've stabilised
                    if !loading_assets.is_empty() && loading_stable_frames >= 3 {
                        loading_started = true;
                    }
                    // timeout: server might have no assets at all
                    if loading_stable_frames >= 30 {
                        loading_started = true;
                    }
                }

                // load one texture per frame to keep the progress bar updating
                if loading_started && loading_index < loading_assets.len() {
                    let (key, path) = loading_assets[loading_index].clone();
                    loading_current = key.clone();
                    load_asset_entry(key, path, &mut playing_state).await;
                    loading_index += 1;
                }

                // transition to Playing when done
                if loading_started && loading_index >= loading_assets.len() {
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                }

                // draw loading screen
                login_screen.draw_loading(
                    loading_assets.len(),
                    loading_index,
                    &loading_current,
                    loading_started,
                );
            }

            GameState::Playing => {
                // F3 → open dev mode (code prompt if not yet verified)
                if is_key_pressed(KeyCode::F3) {
                    playing_state.dev_mode.request_open();
                    if playing_state.dev_mode.is_unlocked() {
                        state = GameState::DevMode;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                        next_frame().await;
                        continue;
                    }
                }

                // handle code prompt while playing (if F3 was pressed but code not yet verified)
                if playing_state.dev_mode.show_code_prompt {
                    set_cursor_grab(false); // show cursor to know ur in prompt mode
                    show_mouse(true);
                    let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                    if let Some(entries) = assets {
                        for (key, path) in entries {
                            load_asset_entry(key, path, &mut playing_state).await;
                        }
                    }
                    playing_state.simulate(&config, maze.as_ref());
                    playing_state.render(&config, &username, player_id, maze.as_ref());
                    if DevModeState::update_code_prompt(&mut playing_state.dev_mode) {
                        state = GameState::DevMode;
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }
                    DevModeState::draw_code_prompt(&playing_state.dev_mode);

                    // if prompt was closed (ESC) without entering dev mode, re-grab cursor for gameplay
                    if !playing_state.dev_mode.show_code_prompt
                        && !matches!(state, GameState::DevMode)
                    {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }

                    next_frame().await;
                    continue;
                }

                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if let Some(entries) = assets {
                    for (key, path) in entries {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }

                if let Some(new_state) = playing_state.process_local_input(&config, maze.as_mut()) {
                    state = new_state;
                    if matches!(state, GameState::Paused) {
                        set_cursor_grab(false);
                        show_mouse(true);
                        playing_state.clear_input();
                    }

                    next_frame().await;
                    continue;
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.send_network(&socket, &server_addr);
                playing_state.render(&config, &username, player_id, maze.as_ref());
            }

            GameState::Paused => {
                // keep world up to date even while paused.
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);

                if let Some(entries) = assets {
                    for (key, path) in entries {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }

                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                if let Some(new_state) =
                    draw_pause_menu(&config, &mut settings_screen, &mut playing_state.dev_mode)
                {
                    state = new_state;
                    if matches!(state, GameState::Playing) {
                        set_cursor_grab(true);
                        show_mouse(false);
                        playing_state.last_mouse_pos = mouse_position();
                    }
                }
            }

            GameState::Settings => {
                clear_background(Color::from_rgba(0, 0, 0, 255));
                settings_screen.draw(
                    &config,
                    &playing_state.weapons,
                    &preset_manager,
                    &playing_state.assets,
                );
                if settings_screen.update(
                    &mut config,
                    &mut playing_state.weapons,
                    &mut preset_manager,
                ) {
                    state = GameState::Paused;
                }
            }

            GameState::DevMode => {
                // F3 or ESC to close dev mode
                if is_key_pressed(KeyCode::F3) {
                    state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                    playing_state.last_mouse_pos = mouse_position();
                    next_frame().await;
                    continue;
                }
                if config.bind_settings.is_pressed() {
                    state = GameState::Paused;
                    next_frame().await;
                    continue;
                }

                // keep world in sync + simulate + render behind the overlay
                let assets = sync_world(&socket, player_id, &mut playing_state, &mut maze);
                if let Some(entries) = assets {
                    for (key, path) in entries {
                        load_asset_entry(key, path, &mut playing_state).await;
                    }
                }
                playing_state.simulate(&config, maze.as_ref());
                playing_state.render(&config, &username, player_id, maze.as_ref());

                // draw dev overlay on top
                draw_dev_overlay(
                    &mut playing_state.dev_mode,
                    &mut playing_state.weapons,
                    &mut playing_state.movement_config,
                    &mut config,
                );
            }

            GameState::Exit => {
                // Send disconnect message to server
                if let Some(ref s) = socket {
                    let disconnect_msg = vec![client::net::protocols::MSG_DISCONNECT];
                    let _ = s.send_to(&disconnect_msg, &server_addr);
                }
                std::process::exit(0);
            }
        }

        next_frame().await;
    }
}

fn sync_world(
    socket: &Option<UdpSocket>,
    player_id: u32,
    playing_state: &mut PlayingState,
    maze: &mut Option<Maze>,
) -> Option<Vec<(String, String)> /* assets to load */> {
    if let Some(ref sock) = socket {
        let updates = poll_updates(sock, player_id);
        if let Some((new_maze, _spawn)) = updates.maze {
            *maze = Some(new_maze);
            // playing_state.position = spawn; // removed because player starts in lobby now
        }
        if let Some(players) = updates.players {
            playing_state.other_players = players;
        }

        if let Some((world_id, spawn)) = updates.world_change {
            playing_state.current_world = world_id;
            playing_state.position = spawn;
            playing_state.movement_state.velocity = Vec3::ZERO;
        }
        if let Some(broken) = updates.broken_walls {
            if let Some(m) = maze.as_mut() {
                for (x, y) in broken {
                    m.force_break_wall(x, y);
                }
            }
        }
        return updates.assets;
    }
    None
}

async fn load_asset_entry(key: String, path: String, playing_state: &mut PlayingState) {
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
