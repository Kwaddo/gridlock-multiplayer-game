use crate::types::maze::Maze;
use crate::types::world::WorldId;
use crate::weapons::combat::{damage_falloff, CombatEntity, EntityKind, HitZone};
use crate::weapons::effects::DamageNumber;
use crate::weapons::recoil::apply_shot_recoil;

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use super::PlayingState;

use crate::weapons::stats::melee::{
    KNIFE_MISS_MOVE_SLOW_DURATION, KNIFE_MISS_PENALTY_SECS, PUNCH_DAMAGE,
};

impl PlayingState {
    /// One shot of the active weapon that just fired: recoil, then each
    /// pellet's ray against surfaces, the dummy, players and bots, with
    /// tracers, bullet holes, hit markers and hit reports.
    pub(super) fn fire_hitscan(&mut self, maze: &mut Option<&mut Maze>) {
        let weapon = self.weapons.active_mut();

        if self.dev_mode.infinite_ammo {
            weapon.ammo = weapon.stats_data.magazine_size;
            weapon.is_reloading = false;
            weapon.reload_timer = 0.0;
        }

        let rng = gen_range(0.0f32, 1.0);
        let fsa_shot = if self.dev_mode.no_recoil {
            false
        } else {
            apply_shot_recoil(weapon, rng)
        };

        let stats = weapon.stats();
        let eye = vec3(
            self.position.x,
            self.position.y + self.movement_state.height,
            self.position.z,
        );
        let base_fwd = vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );

        if !fsa_shot && !self.dev_mode.no_recoil {
            let pattern = stats.recoil_pattern;
            let idx = weapon.shot_index.saturating_sub(1) as usize;
            let (ptrn_pitch, ptrn_yaw) = if pattern.is_empty() {
                (stats.recoil_vertical, 0.0)
            } else {
                pattern[idx.min(pattern.len() - 1)]
            };

            let rand = (stats.pattern_randomness
                + (idx as f32) * stats.randomness_growth)
                .clamp(0.0, 1.0);

            self.pitch += ptrn_pitch;
            let ptrn_h = ptrn_yaw * stats.recoil_horizontal;
            let rand_h = (rng - 0.5) * 2.0 * stats.recoil_horizontal;
            self.yaw += ptrn_h * (1.0 - rand) + rand_h * rand;
            self.pitch = self.pitch.clamp(-1.55, 1.55);
        }

        let right_vec = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
        let up_vec = right_vec.cross(base_fwd).normalize();

        let damage = if weapon.id == crate::weapons::stats::WeaponId::Knife {
            PUNCH_DAMAGE
        } else {
            stats.damage
        };
        let range = stats.bullet_range;
        let spread = weapon.computed_spread;
        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        let mut total_dmg = 0.0f32;
        let mut any_headshot = false;
        let mut pellet_mask: u16 = 0;
        // (victim id, damage, backstab, headshot) per player hit this shot; a shot has
        // at most a few pellets, so a linear scan beats two hash maps.
        // headshot = any pellet hit their head, before this only the hit marker knew
        let mut player_hits: Vec<(u32, f32, bool, bool)> = Vec::new();

        for i in 0..stats.pellet_count as usize {
            // per-pellet nearest-target (reset for each pellet)
            let mut closest: Option<(EntityKind, usize, HitZone, f32)> = None;

            let mut fwd = base_fwd;
            if spread > 0.0001 {
                if let Some(ptrn) = stats.pellet_spread {
                    let (angle, radius): (f32, f32) =
                        ptrn.get(i).copied().unwrap_or((0.0, 0.0));
                    let deviation = spread * radius;
                    fwd = (fwd
                        + right_vec * (angle.cos() * deviation)
                        + up_vec * (angle.sin() * deviation))
                        .normalize();
                } else {
                    let angle = gen_range(0.0f32, std::f32::consts::TAU);
                    let radius = gen_range(0.0f32, spread);
                    fwd = (fwd
                        + right_vec * (angle.cos() * radius)
                        + up_vec * (angle.sin() * radius))
                        .normalize();
                }
            }

            // track whether bullet hit a surface (wall/prop/floor/ceiling/light)
            let mut surface_hit = false;
            let mut surface_normal: Option<Vec3> = None;
            let mut hit_wall_cell: Option<(u8, u8)> = None;
            // mirrors (any world) are checked first: the world is then only
            // tested up to the nearest mirror, so nothing behind a mirror
            // gets hit (no wall damage, no hole, no light broken).
            let mirror_dist =
                crate::world::mirror::nearest_mirror_hit(&self.mirrors, eye, fwd, range);
            let world_range = mirror_dist.unwrap_or(range);
            let world_dist = if self.current_world == WorldId::Maze {
                // gather all candidate hits
                let wall_hit = maze
                    .as_ref()
                    .and_then(|m| m.ray_test_walls_with_normal(eye, fwd, world_range));
                let prop_hit = maze
                    .as_ref()
                    .and_then(|m| m.ray_test_props_with_normal(eye, fwd, world_range));
                let fc_hit = maze.as_ref().and_then(|m| {
                    m.ray_test_floor_ceiling_with_normal(eye, fwd, world_range)
                });
                let light_hit = maze.as_ref().and_then(|m| {
                    m.ray_test_ceiling_lights_with_normal(
                        eye,
                        fwd,
                        world_range,
                        &self.destroyed_lights,
                    )
                });

                // collect all hits as (distance, normal, Option<wall_cell>, is_light_cell)
                let candidates: [Option<(
                    f32,
                    Vec3,
                    Option<(u8, u8)>,
                    Option<(u8, u8)>,
                )>; 4] = [
                    wall_hit.map(|((wx, wy), d, n)| (d, n, Some((wx, wy)), None)),
                    prop_hit.map(|(d, n)| (d, n, None, None)),
                    fc_hit.map(|(d, n)| (d, n, None, None)),
                    light_hit.map(|((lx, ly), d, n)| (d, n, None, Some((lx, ly)))),
                ];

                // pick the closest; on equal distance the earlier candidate
                // wins, as the stable sort this replaces did
                let nearest = candidates.into_iter().flatten().fold(
                    None,
                    |best: Option<(f32, Vec3, Option<(u8, u8)>, Option<(u8, u8)>)>, c| {
                        match best {
                            Some(b) if !(c.0 < b.0) => Some(b),
                            _ => Some(c),
                        }
                    },
                );

                if let Some((dist, norm, wall_cell, light_cell)) = nearest {
                    // if closest hit is a wall, damage it
                    if let Some((wx, wy)) = wall_cell {
                        if let Some(m) = maze.as_mut() {
                            if m.damage_wall(wx, wy, stats.damage) {
                                if !self.pending_broken_walls.contains(&(wx, wy)) {
                                    self.pending_broken_walls.push((wx, wy));
                                }
                            }
                        }
                        hit_wall_cell = Some((wx, wy));
                    }
                    // if closest hit is a light, destroy it
                    if let Some((lx, ly)) = light_cell {
                        self.destroyed_lights.insert((lx, ly));
                    }
                    surface_hit = true;
                    surface_normal = Some(norm);
                    Some(dist)
                } else {
                    None
                }
            } else if self.current_world == WorldId::Lobby {
                // lobby: test against lobby cubes, console, ground
                if let Some((dist, n)) =
                    crate::world::lobby::ray_test_lobby_with_normal(eye, fwd, world_range)
                {
                    surface_hit = true;
                    surface_normal = Some(n);
                    Some(dist)
                } else {
                    None
                }
            } else {
                None
            };
            // nothing in the world was closer, but a mirror was: the bullet
            // stops at the glass. No normal = no bullet hole.
            let closest_blocking_dist = match (world_dist, mirror_dist) {
                (Some(d), _) => Some(d),
                (None, Some(md)) => {
                    surface_hit = true;
                    Some(md)
                }
                (None, None) => None,
            };

            let tracer_start = eye + fwd * 1.5 + vec3(0.0, -0.025, 0.0);
            let tracer_end = if let Some(dist) = closest_blocking_dist {
                eye + fwd * dist
            } else {
                eye + fwd * range
            };

            // when bullet hits a wall, set tracer lifetime to 0 so it disappears
            // on impact rather than lingering
            let tracer_lifetime = if surface_hit {
                0.0
            } else {
                stats.tracer_lifetime
            };

            self.tracers.push(crate::weapons::effects::BulletTracer {
                start: tracer_start,
                end: tracer_end,
                lifetime: tracer_lifetime,
                speed: stats.bullet_speed,
                progress: 0.0,
                tracer_length: stats.tracer_length,
                tracer_color: stats.tracer_color,
                tracer_thickness: stats.tracer_thickness,
            });

            // spawn bullet hole on surface hit
            if surface_hit {
                if let (Some(dist), Some(normal)) =
                    (closest_blocking_dist, surface_normal)
                {
                    let hit_point = eye + fwd * dist;
                    // apply global scale to the configured radius so the
                    // on‑screen decals are less intrusive.  The stats values
                    // can remain as before (e.g. sniper still has a larger
                    // base value) but the final radius is shrunk uniformly.
                    self.bullet_holes.push_back(crate::weapons::effects::BulletHole {
                        position: hit_point + normal * 0.002,
                        normal,
                        lifetime: stats.bullet_hole_lifetime,
                        max_lifetime: stats.bullet_hole_lifetime,
                        size: stats.bullet_hole_size
                            * crate::weapons::effects::BULLET_HOLE_SCALE,
                        color: stats.bullet_hole_color,
                        wall_cell: hit_wall_cell,
                    });
                    // cap total bullet holes to avoid memory bloat
                    if self.bullet_holes.len() > 200 {
                        self.bullet_holes.pop_front();
                    }
                }
            }

            // check target dummy first
            if let Some((zone, dist)) = self.target_dummy.ray_test(eye, fwd, range) {
                let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                if !blocked {
                    let is_closer = closest.map_or(true, |(_, _, _, d)| dist < d);
                    if is_closer {
                        closest = Some((EntityKind::Dummy, 0, zone, dist));
                    }
                }
            }

            // check other players (client-side prediction / hit registration)
            for (idx, other) in self.other_players.iter().enumerate() {
                if !other.is_alive() {
                    continue;
                }
                if let Some((zone, dist)) = other.ray_test(eye, fwd, range) {
                    let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                    if !blocked {
                        let is_closer = closest.map_or(true, |(_, _, _, d)| dist < d);
                        if is_closer {
                            closest = Some((EntityKind::Player, idx, zone, dist));
                        }
                    }
                }
            }

            // check practice bots
            for (idx, bot) in self.practice_bots.iter().enumerate() {
                if !bot.is_alive() {
                    continue;
                }
                if let Some((zone, dist)) = bot.ray_test(eye, fwd, range) {
                    let blocked = closest_blocking_dist.map_or(false, |d| d < dist);
                    if !blocked {
                        let is_closer = closest.map_or(true, |(_, _, _, d)| dist < d);
                        if is_closer {
                            closest = Some((EntityKind::Bot, idx, zone, dist));
                        }
                    }
                }
            }

            if let Some((kind, idx, zone, dist)) = closest {
                let mult = if zone == HitZone::Head {
                    stats.headshot_mult
                } else {
                    zone.damage_mult()
                };
                let falloff = damage_falloff(dist, stats);
                let dmg = damage * mult * falloff;
                let dealt_dmg = dmg;

                match kind {
                    EntityKind::Dummy => {
                        self.target_dummy.take_damage(dmg);
                    }
                    EntityKind::Player => {
                        if let Some(victim_id) = self.other_players[idx].entity_id() {
                            let slot = match player_hits.iter().position(|h| h.0 == victim_id) {
                                Some(i) => i,
                                None => {
                                    player_hits.push((victim_id, 0.0, false, false));
                                    player_hits.len() - 1
                                }
                            };
                            player_hits[slot].1 += dmg;
                            let victim_pos = Vec3::new(
                                self.other_players[idx].x,
                                self.other_players[idx].y,
                                self.other_players[idx].z,
                            );
                            let victim_yaw = self.other_players[idx].yaw;
                            let atk_dir = (self.position - victim_pos).normalize_or_zero();
                            let fwd = Vec3::new(victim_yaw.cos(), 0.0, victim_yaw.sin());
                            let is_backstab = atk_dir.dot(fwd) < -0.5;
                            player_hits[slot].2 = is_backstab;
                            player_hits[slot].3 |= zone == HitZone::Head;
                        }
                    }
                    EntityKind::Bot => {
                        self.practice_bots[idx].take_damage(dealt_dmg);
                    }
                    _ => {}
                }

                total_dmg += dealt_dmg;
                if zone == HitZone::Head {
                    any_headshot = true;
                }
                pellet_mask |= 1 << i;
            }
        }

        if total_dmg > 0.0 {
            self.hit_marker_timer = 0.35;
            self.hit_marker_headshot = any_headshot;
            self.hit_marker_pellet_mask = pellet_mask;
            self.damage_numbers.push(DamageNumber {
                amount: total_dmg,
                is_headshot: any_headshot,
                screen_x: cx + gen_range(-20.0f32, 20.0),
                screen_y: cy - 30.0,
                lifetime: 1.0,
            });
        }

        // Send hit reports in all worlds; server enforces lobby combat rules.
        for (victim_id, damage, is_backstab, is_headshot) in player_hits {
            if damage > 0.0 {
                self.pending_hits.push((victim_id, damage, is_backstab, is_headshot));
            }
        }

        if weapon.id == crate::weapons::stats::WeaponId::Knife && total_dmg <= 0.0 {
            self.knife_attack_lock_timer += KNIFE_MISS_PENALTY_SECS;
            self.knife_miss_slow_timer = KNIFE_MISS_MOVE_SLOW_DURATION;
        }
    }
}
