use super::super::PlayingState;

impl PlayingState {
    /// Which animation your own body plays, worked out from your own movement
    /// the same way other players' animations are worked out from theirs.
    pub(crate) fn self_avatar_skin(&self) -> crate::types::player::PlayerSkin {
        use crate::types::animation_state::AnimState;
        let m = &self.movement_state;
        // where you'd have been 0.1 s ago at this speed = your move direction
        // no keys held = standing, the blend eases you into idle
        // before this it went by leftover velocity, so a tap of W walked for ~0.5 s while you slid to a stop
        let prev = if self.move_dir == macroquad::prelude::Vec3::ZERO {
            self.position
        } else {
            self.position - m.velocity * 0.1
        };
        let state = AnimState::from_movement(
            prev.x,
            prev.z,
            self.position.x,
            self.position.z,
            self.yaw,
            m.is_sprinting,
            self.weapons.active().is_aiming,
            m.is_crouching,
            m.is_prone,
            m.is_sliding,
            false
        );
        let skin = AnimState::skin_from_state(state, None);
        // your own slow walk is known for sure, no guessing from speed like for other players
        if m.is_slow_walking && skin == crate::types::player::PlayerSkin::RifleWalk {
            crate::types::player::PlayerSkin::SlowWalkForward
        } else {
            skin
        }
    }

    // you in the mirror, the body update_self_avatar keeps in sync
    // before this it built a brand new throwaway body every frame, so its animation clock reset every frame
    // only the mirror pass draws it, in first person you never see your own body
    pub(crate) fn draw_self_avatar(&self) {
        if self.is_dead {
            return; // the death camera draws your corpse instead
        }
        let muzzle = self.draw_player_model(&self.self_body);
        // your own shots flash on your reflection's gun, same flash as bots, same 0.08 s as the first person one
        // before this you fired in the mirror with no flash at all
        let flash = self.weapons.active().muzzle_flash_timer;
        if let (Some(at), true) = (muzzle, flash > 0.0) {
            super::tracers::draw_muzzle_flash(at, (flash / 0.08).min(1.0));
        }
    }

    // which death you play, from the stance you died in
    // before this draw_death_corpse worked it out every frame while drawing
    fn death_skin(&self) -> crate::types::player::PlayerSkin {
        use crate::types::anim_chooser::{choose_death, stance};
        // the kill feed's pick if it came in, otherwise the table's default for your stance
        self.my_death.unwrap_or_else(|| {
            let m = &self.movement_state;
            choose_death(stance(m.is_prone, m.is_crouching), self.position, self.yaw, None, false, 0)
        })
    }

    // keep your one third person body in sync with you, alive = copy you, dead = the corpse
    // before this the mirror self and the corpse were thrown away and rebuilt every frame
    // so they could never remember an animation clock

    pub(crate) fn update_self_avatar(&mut self, delta: f32) {
        use crate::screens::playing::state::DeathCameraPhase;
        let corpse = match self.death_camera_phase {
            | DeathCameraPhase::ZoomOut { corpse_pos, corpse_yaw, .. }
            | DeathCameraPhase::Watching { corpse_pos, corpse_yaw, .. } =>
                Some((corpse_pos, corpse_yaw)),
            _ => None,
        };

        let (pos, yaw, skin) = match corpse {
            Some((pos, yaw)) => (pos, yaw, self.death_skin()),
            None => (self.position, self.yaw, self.self_avatar_skin()),
        };

        let aiming = self.weapons.active().is_aiming;
        let weapon = self.weapons.active().stats().name;
        // look up/down while alive, the corpse lies level
        let pitch = if corpse.is_some() { 0.0 } else { self.pitch };

        let body = &mut self.self_body;
        body.render_pitch = pitch; // the gun tilts with it too
        body.render_x = pos.x;
        body.render_y = pos.y;
        body.render_z = pos.z;
        body.render_yaw = yaw;
        body.skin_type = skin;
        body.is_sprinting = self.movement_state.is_sprinting;
        body.is_crouching = self.movement_state.is_crouching;
        body.is_prone = self.movement_state.is_prone;
        body.is_ads = aiming;
        if body.current_weapon != weapon {
            body.current_weapon = weapon.to_string(); // only when weapon changes, not every frame
        }
        body.anim.tick(skin, delta, &self.animated_models, pos, pitch);
    }
}
