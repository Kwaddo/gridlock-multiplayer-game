pub(super) fn in_rect(mouse: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h
}
