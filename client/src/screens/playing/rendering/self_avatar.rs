use super::super::PlayingState;

impl PlayingState {
    /// Which animation your own body plays, worked out from your own movement
    /// the same way other players' animations are worked out from theirs.
    pub(crate) fn self_avatar_skin(&self) -> crate::types::player::PlayerSkin {
        use crate::types::animation_state::AnimState;
        let m = &self.movement_state;
        // where you'd have been 0.1 s ago at this speed = your move direction
        let prev = self.position - m.velocity * 0.1;
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
            false,
        );
        AnimState::skin_from_state(state, None)
    }

    /// Your own body as others see it, for the mirror. Built fresh every frame
    /// from your local state (no network lag). Only the mirror pass draws it:
    /// in first person you never see your own body.
    pub(crate) fn draw_self_avatar(&self, player_id: u32) {
        if self.is_dead {
            return; // the death camera draws your corpse instead
        }
        let mut me =
            crate::types::player::OtherPlayer::new(player_id, String::new(), self.current_world);
        me.render_x = self.position.x;
        me.render_y = self.position.y;
        me.render_z = self.position.z;
        me.render_yaw = self.yaw;
        me.skin_type = self.self_avatar_skin();
        me.is_sprinting = self.movement_state.is_sprinting;
        me.is_crouching = self.movement_state.is_crouching;
        me.is_prone = self.movement_state.is_prone;
        me.is_ads = self.weapons.active().is_aiming;
        me.current_weapon = self.weapons.active().stats().name.to_string();
        self.draw_player_model(&me);
    }
}
