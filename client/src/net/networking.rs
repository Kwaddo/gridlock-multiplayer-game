use std::net::UdpSocket;

use macroquad::prelude::*;

use crate::net::protocols::{
    EVT_JUMP_START, EVT_LANDED, FLAG_ADS, FLAG_AIRBORNE, FLAG_CROUCHING, FLAG_SPRINTING,
    MSG_ASSETS, MSG_DAMAGE, MSG_DEATH, MSG_KILL_FEED, MSG_MATCH_COUNTDOWN, MSG_MATCH_END,
    MSG_MATCH_JOINED, MSG_MATCH_TIMER, MSG_MAZE, MSG_PLAYER_LIST, MSG_POSITION, MSG_RESPAWN,
    MSG_START_DENIED, MSG_STATE, MSG_WALL_BROKEN, MSG_WORLD_CHANGE,
};
use crate::types::maze::{CeilingLightInfo, Maze, Prop, PropKind, Room};
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;

pub const MAZE_CELL_SIZE: f32 = 1.8;
pub const MAZE_WALL_HEIGHT: f32 = 2.4;

pub struct NetUpdate {
    pub maze: Option<(Maze, Vec3)>,
    pub players: Option<Vec<OtherPlayer>>,
    pub world_change: Option<(WorldId, Vec3, Option<u8>)>, // (world, spawn, freeze_secs)
    pub assets: Option<Vec<(String, String)>>,             // (key, path)
    pub broken_walls: Option<Vec<(u8, u8)>>,
    pub damage_taken: Option<(f32, Option<u32>)>, // (damage amount, attacker_id)
    pub kill_feed: Option<Vec<(u32, u32, String)>>, // (killer_id, victim_id, weapon)
    pub respawn_pos: Option<Vec3>,                // (x, y, z)
    pub respawn_fallback: Option<bool>,           // true if server fell back to teleporter
    pub lobby_console_message: Option<String>,
    pub match_join_announce: Option<String>,
    pub match_start_countdown: Option<u8>,
    pub match_time_left: Option<u8>,
    pub match_end_winner: Option<String>,
    pub match_end_countdown: Option<u8>,
    pub player_list: Option<Vec<(u32, String, u32, WorldId, bool)>>,
    pub self_kills: Option<u32>,
    pub self_gun_level: Option<u8>,
    pub match_mode: Option<u8>,
    pub self_freeze_secs: Option<u8>,
    pub death: bool, // You died

    // true if poll_updates read at least one packet from the socket during this call
    pub received_any: bool,
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
        respawn_fallback: None,
        lobby_console_message: None,
        match_join_announce: None,
        match_start_countdown: None,
        match_time_left: None,
        match_end_winner: None,
        match_end_countdown: None,
        player_list: None,
        self_kills: None,
        self_gun_level: None,
        match_mode: None,
        death: false,
        received_any: false,
        self_freeze_secs: None,
    };

    let mut recv_buf: [u8; 8192] = [0; 8192];

    while let Ok((size, _)) = socket.recv_from(&mut recv_buf) {
        // mark that we received at least one packet during this poll
        update.received_any = true;

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
                let player_count =
                    u32::from_be_bytes([recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4]])
                        as usize;
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

                    // new: kills (u32)
                    let kills = u32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;

                    // new: gun level (1 = first weapon)
                    let gun_level = recv_buf[offset];
                    offset += 1;

                    // health and max_health (f32 each)
                    let hp = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;
                    let max_hp = f32::from_be_bytes([
                        recv_buf[offset],
                        recv_buf[offset + 1],
                        recv_buf[offset + 2],
                        recv_buf[offset + 3],
                    ]);
                    offset += 4;

                    let name_len = recv_buf[offset] as usize;
                    offset += 1;
                    let name =
                        String::from_utf8_lossy(&recv_buf[offset..offset + name_len]).to_string();
                    offset += name_len;
                    // new: alive flag (1 = alive, 0 = dead)
                    let mut alive = true;
                    if offset < size {
                        alive = recv_buf[offset] != 0;
                        offset += 1;
                    }

                    // new: remaining post-teleport freeze seconds (0 = none)
                    let mut freeze_secs: u8 = 0;
                    if offset < size {
                        freeze_secs = recv_buf[offset];
                        offset += 1;
                    }

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
                        other_player.kills = kills;
                        other_player.gun_level = gun_level;
                        other_player.hp = hp;
                        other_player.max_hp = max_hp;
                        // Only add other players who are alive; dead players are
                        // represented by the alive flag and will be removed by
                        // the client when appropriate so offscreen deaths are
                        // reliably propagated.
                        if alive {
                            new_players.push(other_player);
                        }
                    } else {
                        // update own-kills value for local UI/scoreboard
                        update.self_kills = Some(kills);
                        update.self_gun_level = Some(gun_level);
                        // If the server reports we are dead in the state packet,
                        // mark death so UI/state paths handle it even if MSG_DEATH
                        // was missed.
                        if !alive {
                            update.death = true;
                        }
                        // record authoritative post-teleport freeze so client can
                        // apply movement/weapon locks even if the MSG_WORLD_CHANGE
                        // packet was lost while joining the match.
                        update.self_freeze_secs = Some(freeze_secs);
                    }
                }
                update.players = Some(new_players);
            }
            MSG_WORLD_CHANGE => {
                if size >= 14 {
                    let world_id = recv_buf[1];
                    let world = WorldId::from_u8(world_id).unwrap_or(WorldId::Lobby);
                    let x =
                        f32::from_be_bytes([recv_buf[2], recv_buf[3], recv_buf[4], recv_buf[5]]);
                    let y =
                        f32::from_be_bytes([recv_buf[6], recv_buf[7], recv_buf[8], recv_buf[9]]);
                    let z = f32::from_be_bytes([
                        recv_buf[10],
                        recv_buf[11],
                        recv_buf[12],
                        recv_buf[13],
                    ]);
                    // optional trailing bytes: [14]=gamemode, [15]=freeze_secs
                    let mut freeze_secs: Option<u8> = None;
                    if size >= 15 {
                        update.match_mode = Some(recv_buf[14]);
                    }
                    if size >= 16 {
                        freeze_secs = Some(recv_buf[15]);
                    }
                    update.world_change = Some((world, vec3(x, y, z), freeze_secs));
                }
            }
            MSG_START_DENIED => {
                // server says: match running and full (or otherwise deny start)
                update.lobby_console_message = Some("Match already running and full".to_string());
            }
            MSG_MATCH_JOINED => {
                // format: [MSG_MATCH_JOINED][player_id:4][name_len:1][name bytes]
                if size >= 6 {
                    let name_len = recv_buf[5] as usize;
                    if size >= 6 + name_len {
                        let name = String::from_utf8_lossy(&recv_buf[6..6 + name_len]).to_string();
                        update.match_join_announce = Some(format!("{} joined the match", name));
                    }
                }
            }
            MSG_MATCH_COUNTDOWN => {
                // format: [MSG_MATCH_COUNTDOWN][seconds:1][optional:mode:1]
                if size >= 2 {
                    let secs = recv_buf[1];
                    update.match_start_countdown = Some(secs);
                    if size >= 3 {
                        update.match_mode = Some(recv_buf[2]);
                    }
                }
            }
            MSG_MATCH_TIMER => {
                // format: [MSG_MATCH_TIMER][seconds:1] -- authoritative remaining match time
                if size >= 2 {
                    let secs = recv_buf[1];
                    update.match_time_left = Some(secs);
                }
            }
            MSG_MATCH_END => {
                // format: [MSG_MATCH_END][winner_id:4][name_len:1][name bytes][countdown?:1]
                if size >= 6 {
                    let name_len = recv_buf[5] as usize;
                    if size >= 6 + name_len {
                        let name = String::from_utf8_lossy(&recv_buf[6..6 + name_len]).to_string();
                        update.match_end_winner = Some(name.clone());
                        // optional trailing countdown byte
                        if size >= 6 + name_len + 1 {
                            let cd = recv_buf[6 + name_len];
                            update.match_end_countdown = Some(cd);
                        }
                    }
                }
            }
            MSG_PLAYER_LIST => {
                // format: [MSG_PLAYER_LIST][count:1][entries...]
                // entry: [id:4][name_len:1][name bytes][kills:4][world:1][alive:1]
                if size >= 2 {
                    let count = recv_buf[1] as usize;
                    let mut offset = 2usize;
                    let mut list = Vec::new();
                    for _ in 0..count {
                        if offset + 6 > size {
                            break;
                        }
                        let id = u32::from_be_bytes([
                            recv_buf[offset],
                            recv_buf[offset + 1],
                            recv_buf[offset + 2],
                            recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let name_len = recv_buf[offset] as usize;
                        offset += 1;
                        if offset + name_len + 6 > size {
                            break;
                        }
                        let name = String::from_utf8_lossy(&recv_buf[offset..offset + name_len])
                            .to_string();
                        offset += name_len;
                        let kills = u32::from_be_bytes([
                            recv_buf[offset],
                            recv_buf[offset + 1],
                            recv_buf[offset + 2],
                            recv_buf[offset + 3],
                        ]);
                        offset += 4;
                        let world_id = recv_buf[offset];
                        offset += 1;
                        let alive = recv_buf[offset] != 0;
                        offset += 1;
                        let world = WorldId::from_u8(world_id).unwrap_or(WorldId::Lobby);
                        list.push((id, name, kills, world, alive));
                    }
                    update.player_list = Some(list);
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
                        let key = String::from_utf8_lossy(&recv_buf[offset..offset + key_len])
                            .to_string();
                        offset += key_len;
                        let path_len = recv_buf[offset] as usize;
                        offset += 1;
                        let path = String::from_utf8_lossy(&recv_buf[offset..offset + path_len])
                            .to_string();
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
                    let damage =
                        f32::from_be_bytes([recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4]]);
                    let attacker_id = if size >= 9 {
                        Some(u32::from_be_bytes([
                            recv_buf[5],
                            recv_buf[6],
                            recv_buf[7],
                            recv_buf[8],
                        ]))
                    } else {
                        None
                    };
                    update.damage_taken = Some((damage, attacker_id));
                }
            }
            MSG_KILL_FEED => {
                if size >= 10 {
                    let killer_id =
                        u32::from_be_bytes([recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4]]);
                    let victim_id =
                        u32::from_be_bytes([recv_buf[5], recv_buf[6], recv_buf[7], recv_buf[8]]);
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
                    let x =
                        f32::from_be_bytes([recv_buf[1], recv_buf[2], recv_buf[3], recv_buf[4]]);
                    let y =
                        f32::from_be_bytes([recv_buf[5], recv_buf[6], recv_buf[7], recv_buf[8]]);
                    let z =
                        f32::from_be_bytes([recv_buf[9], recv_buf[10], recv_buf[11], recv_buf[12]]);
                    update.respawn_pos = Some(vec3(x, y, z));
                    if size >= 14 {
                        let kind = recv_buf[13];
                        update.respawn_fallback = Some(kind == 1);
                    }
                }
            }
            _ => {}
        }
    }

    update
}
