use super::{LoginField, LoginScreen};
use crate::screens::login::colors::*;
use crate::screens::login::util::in_rect;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

impl LoginScreen {
    pub(super) fn draw_play_tab(&mut self, px: f32, cy: f32, pw: f32, _ch: f32, pad: f32) {
        let field_w = pw - pad * 2.0;
        let field_h = 34.0;
        let fx = px + pad;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        let ip_y = cy + 24.0;
        let ip_active = matches!(self.active_field, LoginField::Ip);
        draw_text_ex(
            "SERVER ADDRESS",
            fx,
            ip_y,
            text_params(13, if ip_active { ACCENT } else { TEXT_LABEL }),
        );
        let ip_fy = ip_y + 10.0;
        self.draw_input(
            fx,
            ip_fy,
            field_w,
            field_h,
            &self.ip_input.clone(),
            ip_active,
            "127.0.0.1:7878",
        );
        if in_rect(mouse, fx, ip_fy, field_w, field_h) && clicked {
            self.active_field = LoginField::Ip;
        }

        let nm_y = ip_fy + field_h + 26.0;
        let nm_active = matches!(self.active_field, LoginField::Username);
        draw_text_ex(
            "USERNAME",
            fx,
            nm_y,
            text_params(13, if nm_active { ACCENT } else { TEXT_LABEL }),
        );
        let nm_fy = nm_y + 10.0;
        self.draw_input(
            fx,
            nm_fy,
            field_w,
            field_h,
            &self.username_input.clone(),
            nm_active,
            "Player",
        );
        if in_rect(mouse, fx, nm_fy, field_w, field_h) && clicked {
            self.active_field = LoginField::Username;
        }

        let bw = 220.0;
        let bh = 42.0;
        let bx = px + (pw - bw) / 2.0;
        let by = nm_fy + field_h + 32.0;
        let hover = in_rect(mouse, bx, by, bw, bh);
        draw_rectangle(bx, by, bw, bh, if hover { ACCENT_HOVER } else { ACCENT });
        let ctxt = "CONNECT";
        let ctw = measure_game_text(ctxt, 20);
        draw_text_ex(
            ctxt,
            bx + (bw - ctw) / 2.0,
            by + 28.0,
            text_params(20, Color::new(0.05, 0.05, 0.05, 1.0)),
        );
        if hover && clicked {
            self.try_connect();
        }

        if !self.error_msg.is_empty() {
            let ey = by + bh + 16.0;
            let ew = measure_game_text(&self.error_msg, 15);
            draw_text_ex(
                &self.error_msg,
                px + (pw - ew) / 2.0,
                ey,
                text_params(15, Color::new(1.0, 0.30, 0.30, 1.0)),
            );
        }
    }
}
