use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::{
    in_rect, ACCENT, ACCENT_HOVER, INPUT_BG, INPUT_BORDER, TEXT_LABEL, LoginScreen,
};

impl LoginScreen {
    pub(super) fn draw_play_tab(&mut self, px: f32, cy: f32, pw: f32, _ch: f32, pad: f32) {
        let field_w = pw - pad * 2.0;
        let field_h = 34.0;
        let fx = px + pad;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        let ip_y = cy + 24.0;
        let ip_active = matches!(self.active_field, crate::core::game_state::LoginField::Ip);
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
            self.active_field = crate::core::game_state::LoginField::Ip;
        }

        let nm_y = ip_fy + field_h + 26.0;
        let nm_active = matches!(
            self.active_field,
            crate::core::game_state::LoginField::Username
        );
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
            self.active_field = crate::core::game_state::LoginField::Username;
        }

        let mq_y = nm_fy + field_h + 18.0;
        let mq_label = format!("Model Quality: {}", self.model_quality.label());
        let mq_w = measure_game_text(&mq_label, 14) + 24.0;
        let mq_h = 28.0;
        let mq_x = px + (pw - mq_w) / 2.0;
        let mq_hover = in_rect(mouse, mq_x, mq_y, mq_w, mq_h);
        draw_rectangle(
            mq_x,
            mq_y,
            mq_w,
            mq_h,
            if mq_hover {
                Color::new(0.14, 0.16, 0.26, 1.0)
            } else {
                Color::new(0.08, 0.10, 0.18, 1.0)
            },
        );
        draw_rectangle_lines(mq_x, mq_y, mq_w, mq_h, 1.0, INPUT_BORDER);
        let mq_tw = measure_game_text(&mq_label, 14);
        draw_text_ex(
            &mq_label,
            mq_x + (mq_w - mq_tw) / 2.0,
            mq_y + 19.0,
            text_params(14, TEXT_LABEL),
        );
        if mq_hover && clicked {
            self.model_quality = self.model_quality.toggle();
        }

        let bw = 220.0;
        let bh = 42.0;
        let bx = px + (pw - bw) / 2.0;
        let by = mq_y + mq_h + 14.0;
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

    fn draw_input(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        active: bool,
        placeholder: &str,
    ) {
        draw_rectangle(x, y, w, h, INPUT_BG);
        let bc = if active { ACCENT } else { INPUT_BORDER };
        draw_rectangle_lines(x, y, w, h, if active { 2.0 } else { 1.0 }, bc);

        if text.is_empty() && !active {
            draw_text_ex(
                placeholder,
                x + 10.0,
                y + 23.0,
                text_params(17, Color::new(0.30, 0.32, 0.42, 0.5)),
            );
        } else {
            draw_text_ex(text, x + 10.0, y + 23.0, text_params(17, WHITE));
            if active {
                let visible = (self.anim_time * 2.5) as i32 % 2 == 0;
                if visible {
                    let tw = measure_game_text(text, 17);
                    draw_rectangle(x + 10.0 + tw + 2.0, y + 6.0, 2.0, h - 12.0, ACCENT);
                }
            }
        }
    }
}
