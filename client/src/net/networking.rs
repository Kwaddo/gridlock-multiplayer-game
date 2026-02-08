use std::net::UdpSocket;

use macroquad::prelude::*;

use crate::net::protocols::{ MSG_MAZE, MSG_POSITION, MSG_STATE, MSG_WORLD_CHANGE, MSG_ASSETS };
use crate::types::maze::Maze;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;

pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WALL_HEIGHT: f32 = 2.4;

pub struct NetUpdate {
    pub maze: Option<(Maze, Vec3)>,
    pub players: Option<Vec<OtherPlayer>>,
    pub world_change: Option<(WorldId, Vec3)>,
    pub assets: Option<Vec<(String, String)>>, // (key, path)
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
    };

    let mut recv_buf: [u8; 2048] = [0; 2048];

    while let Ok((size, _)) = socket.recv_from(&mut recv_buf) {
        if size == 0 {
            continue;
        }
        match recv_buf[0] {
            MSG_MAZE => {
                if size >= 3 {
                    let width = recv_buf[1];
                    let height = recv_buf[2];
                    let cell_count = (width as usize) * (height as usize);
                    if size >= 3 + cell_count {
                        let mut cells = vec![0u8; cell_count];
                        cells.copy_from_slice(&recv_buf[3..3 + cell_count]);
                        let maze = Maze {
                            width,
                            height,
                            cells,
                            cell_size: MAZE_CELL_SIZE,
                            wall_height: MAZE_WALL_HEIGHT,
                        };
                        let spawn = maze.spawn_point();
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
                    let name_len = recv_buf[offset] as usize;
                    offset += 1;
                    let name = String::from_utf8_lossy(
                        &recv_buf[offset..offset + name_len]
                    ).to_string();
                    offset += name_len;

                    if id != player_id {
                        let other_player = OtherPlayer {
                            id,
                            x,
                            y,
                            z,
                            yaw,
                            username: name,
                            skin: None,
                            color: OtherPlayer::color_for_id(id),
                            world,
                        };
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
            _ => {}
        }
    }

    update
}
