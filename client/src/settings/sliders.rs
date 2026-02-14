use crate::core::GameConfig;
use crate::settings::fov;
use crate::settings::sensitivity;

pub fn update_sliders(
    config: &mut GameConfig,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    panel_top: f32,
    mouse: (f32, f32),
    clicked: bool,
    held: bool,
) {
    let row_y = panel_top + 50.0;
    sensitivity::update(
        &mut config.sensitivity,
        slider_x,
        slider_w,
        btn_size,
        row_y,
        mouse,
        clicked,
        held,
    );

    let row_y2 = row_y + 60.0;
    fov::update(
        &mut config.fov,
        slider_x,
        slider_w,
        btn_size,
        row_y2,
        mouse,
        clicked,
        held,
    );
}

pub fn draw_sliders(
    config: &GameConfig,
    label_x: f32,
    slider_x: f32,
    slider_w: f32,
    btn_size: f32,
    panel_top: f32,
) {
    let row_y = panel_top + 50.0;
    sensitivity::draw(
        config.sensitivity,
        label_x,
        slider_x,
        slider_w,
        btn_size,
        row_y,
    );

    let row_y2 = row_y + 60.0;
    fov::draw(config.fov, label_x, slider_x, slider_w, btn_size, row_y2);
}
