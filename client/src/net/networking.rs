use std::net::UdpSocket;

use macroquad::prelude::*;

use crate::net::protocols::{
    EVT_JUMP_START,
    EVT_LANDED,
    FLAG_ADS,
    FLAG_AIRBORNE,
    FLAG_CROUCHING,
    FLAG_SPRINTING,
    MSG_ASSETS,
    MSG_DAMAGE,
    MSG_DEATH,
    MSG_KILL_FEED,
    MSG_MAZE,
    MSG_POSITION,
    MSG_RESPAWN,
    MSG_STATE,
    MSG_WALL_BROKEN,
    MSG_WORLD_CHANGE,
};
use crate::types::maze::{ CeilingLightInfo, Maze, Prop, PropKind, Room };
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;

pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WALL_HEIGHT: f32 = 2.4;

pub struct NetUpdate {
    pub maze: Option<(Maze, Vec3)>,
    pub players: Option<Vec<OtherPlayer>>,
    pub world_change: Option<(WorldId, Vec3)>,
    pub assets: Option<Vec<(String, String)>>, // (key, path)
    pub broken_walls: Option<Vec<(u8, u8)>>,
    pub damage_taken: Option<(f32, Option<u32>)>, // (damage amount, attacker_id)
    pub kill_feed: Option<Vec<(u32, u32, String)>>, // (killer_id, victim_id, weapon)
    pub respawn_pos: Option<Vec3>, // (x, y, z)
    pub death: bool, // You died
}

pub fn send_position(socket: &UdpSocket, server_addr: &str, position: Vec3, yaw: f32) {
    let mut pos_msg = vec![MSG_POSITION];
    pos_msg.extend_from_slice(&position.x.to_be_bytes());
    pos_msg.extend_from_slice(&position.y.to_be_bytes());
    pos_msg.extend_from_slice(&position.z.to_be_bytes());
    pos_msg.extend_from_slice(&yaw.to_be_bytes());
    let _ = socket.send_to(&pos_msg, server_addr);
}

pub fn poll_updates(socket: &UdpSocket, player_id: u32) -> NetUpdate {
    let mut update = NetUpdate {
        maze: None,
        players: None,
        world_change: None,
        assets: None,
        broken_walls: None,
        damage_taken: None,
        kill_feed: None,
        respawn_pos: None,
        death: false,
    };

    let mut recv_buf: [u8; 8192] = [0; 8192];

    while let Ok((size, _)) = socket.recv_from(&mut recv_buf) {
        if size == 0 {
            continue;
        }
        match recv_buf[0] {
            MSG_MAZE => {
                if size >= 5 {
                    let width = recv_buf[1];
                    let height = recv_buf[2];
                    let tele_x = recv_buf[3];
                    let tele_y = recv_buf[4];

                    let mut offset = 5;
                    if offset >= size {
                        continue;
                    }

                    let room_count = recv_buf[offset] as usize;
                    offset += 1;
                    let mut rooms = Vec::with_capacity(room_count);
                    for _ in 0..room_count {
                        if offset + 4 > size {
                            break;
                        }
                        rooms.push(Room {
                            x: recv_buf[offset],
                            y: recv_buf[offset + 1],
                            w: recv_buf[offset + 2],
                            h: recv_buf[offset + 3],
                        });
                        offset += 4;
                    }

                    if offset >= size {
                        continue;
                    }

                    let prop_count = recv_buf[offset] as usize;
                    offset += 1;
                    let mut props = Vec::with_capacity(prop_count);
                    for _ in 0..prop_count {
                        if offset + 3 > size {
                            break;
                        }
                        if let Some(kind) = PropKind::from_u8(recv_buf[offset]) {
                            props.push(Prop {
                                kind,
                                x: recv_buf[offset + 1],
                                y: recv_buf[offset + 2],
                            });
                        }
                        offset += 3;
                    }

                    let mut ceiling_lights = Vec::new();
                    if offset < size {
                        let light_count = recv_buf[offset] as usize;
                        offset += 1;
                        ceiling_lights.reserve(light_count);
                        for _ in 0..light_count {
                            if offset + 3 > size {
                                break;
                            }
                            ceiling_lights.push(CeilingLightInfo {
                                x: recv_buf[offset],
                                y: recv_buf[offset + 1],
                                variant: recv_buf[offset + 2],
                            });
                            offset += 3;
                        }
                    }

                    let cell_count = (width as usize) * (height as usize);
                    if size >= offset + cell_count {
                        let mut cells = vec![0u8; cell_count];
                        cells.copy_from_slice(&recv_buf[offset..offset + cell_count]);
                        let mut maze = Maze {
                            width,
                            height,
                            cells,
                            cell_size: MAZE_CELL_SIZE,
                            wall_height: MAZE_WALL_HEIGHT,
                            teleporter: (tele_x, tele_y),
                            rooms,
                            props,
                            ceiling_lights,
                            breakable_walls: std::collections::HashMap::new(),
                            broken_wall_scars: std::collections::HashSet::new(),
                        };
                        maze.spawn_breakable_walls();
                        let spawn = maze.teleporter_world();
                        update.maze = Some((maze, spawn));
                    }
                }
            }
            MSG_STATE => {
                let player_count = u32::from_be_bytes([
                    recv_buf[1],
                    recv_buf[2],
                    recv_buf[3],
                    recv_buf[4],
                ]) as usize;
                let mut offset = 5;
                let mut new_players = Vec::new();
                for _ in 0..player_count {
                    let id = u32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let x = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let y = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let z = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let yaw = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let world_id = recv_buf[offset];
                    offset += 1;
                    let world = WorldId::from_u8(world_id).unwrap_or(WorldId::Lobby);
                    let state_flags = recv_buf[offset];
                    offset += 1;
                    let event_flags = recv_buf[offset];
                    offset += 1;
                    let name_len = recv_buf[offset] as usize;
                    offset += 1;
                    let name = String::from_utf8_lossy(
                        &recv_buf[offset..offset + name_len]
                    ).to_string();
                    offset += name_len;

                    if id != player_id {
                        let mut other_player = OtherPlayer::new(id, name, world);
                        other_player.x = x;
                        other_player.y = y;
                        other_player.z = z;
                        other_player.yaw = yaw;
                        other_player.is_sprinting = (state_flags & FLAG_SPRINTING) != 0;
                        other_player.is_crouching = (state_flags & FLAG_CROUCHING) != 0;
                        other_player.is_ads = (state_flags & FLAG_ADS) != 0;
                        other_player.is_airborne = (state_flags & FLAG_AIRBORNE) != 0;
                        other_player.evt_jump_start = (event_flags & EVT_JUMP_START) != 0;
                        other_player.evt_landed = (event_flags & EVT_LANDED) != 0;
                        new_players.push(other_player);
                    }
                }
                update.players = Some(new_players);
            }
            MSG_WORLD_CHANGE => {
                if size >= 14 {
                    let world_id = recv_buf[1];
                    let world = WorldId::from_u8(world_id).unwrap_or(WorldId::Lobby);
                    let x = f32::from_be_bytes([
                        recv_buf[2],
                        recv_buf[3],
                        recv_buf[4],
                        recv_buf[5],
                    ]);
                    let y = f32::from_be_bytes([
                        recv_buf[6],
                        recv_buf[7],
                        recv_buf[8],
                        recv_buf[9],
                    ]);
                    let z = f32::from_be_bytes([
                        recv_buf[10],
                        recv_buf[11],
                        recv_buf[12],
                        recv_buf[13],
                    ]);
                    update.world_change = Some((world, vec3(x, y, z)));
                }
            }
            MSG_ASSETS => {
                if size >= 3 {
                    let _world_id = recv_buf[1];
                    let num_entries = recv_buf[2] as usize;
                    let mut offset = 3;
                    let mut entries = Vec::new();
                    for _ in 0..num_entries {
                        if offset >= size {
                            break;
                        }
                        let key_len = recv_buf[offset] as usize;
                        offset += 1;
                        let key = String::from_utf8_lossy(
                            &recv_buf[offset..offset + key_len]
                        ).to_string();
                        offset += key_len;
                        let path_len = recv_buf[offset] as usize;
                        offset += 1;
                        let path = String::from_utf8_lossy(
                            &recv_buf[offset..offset + path_len]
                        ).to_string();
                        offset += path_len;
                        entries.push((key, path));
                    }
                    if let Some(ref mut existing) = update.assets {
                        existing.extend(entries);
                    } else {
                        update.assets = Some(entries);
                    }
                }
            }
            MSG_WALL_BROKEN => {
                if size >= 3 {
                    let x = recv_buf[1];
                    let y = recv_buf[2];
                    if let Some(ref mut walls) = update.broken_walls {
                        walls.push((x, y));
                    } else {
                        update.broken_walls = Some(vec![(x, y)]);
                    }
                }
            }
            MSG_DAMAGE => {
                if size >= 5 {
                    let damage = f32::from_be_bytes([
                        recv_buf[1],
                        recv_buf[2],
                        recv_buf[3],
                        recv_buf[4],
                    ]);
                    let attacker_id = if size >= 9 {
                        Some(
                            u32::from_be_bytes([recv_buf[5], recv_buf[6], recv_buf[7], recv_buf[8]])
                        )
                    } else {
                        None
                    };
                    update.damage_taken = Some((damage, attacker_id));
                }
            }
            MSG_KILL_FEED => {
                if size >= 10 {
                    let killer_id = u32::from_be_bytes([
                        recv_buf[1],
                        recv_buf[2],
                        recv_buf[3],
                        recv_buf[4],
                    ]);
                    let victim_id = u32::from_be_bytes([
                        recv_buf[5],
                        recv_buf[6],
                        recv_buf[7],
                        recv_buf[8],
                    ]);
                    let weapon_len = recv_buf[9] as usize;
                    let weapon = if size >= 10 + weapon_len {
                        String::from_utf8_lossy(&recv_buf[10..10 + weapon_len]).to_string()
                    } else {
                        "Unknown".to_string()
                    };
                    if let Some(ref mut feed) = update.kill_feed {
                        feed.push((killer_id, victim_id, weapon));
                    } else {
                        update.kill_feed = Some(vec![(killer_id, victim_id, weapon)]);
                    }
                }
            }
            MSG_DEATH => {
                update.death = true;
            }
            MSG_RESPAWN => {
                if size >= 13 {
                    let x = f32::from_be_bytes([recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4]]);
                    let y = f32::from_be_bytes([recv_buf[5], recv_buf[6], recv_buf[7], recv_buf[8]]);
                    let z = f32::from_be_bytes([recv_buf[9], recv_buf[10], recv_buf[11], recv_buf[12]]);
                    update.respawn_pos = Some(vec3(x, y, z));
                }
            }
            _ => {}
        }
    }

    update
}
