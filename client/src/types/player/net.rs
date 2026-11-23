/// Where another player is and what they are doing, as the server last said
/// (`MSG_STATE`). The body is not drawn here: see `RenderSmoothing`.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    // look up/down from the server (+ = up), the body bends toward it
    // before this pitch never reached other players, so their bodies always looked level
    pub pitch: f32,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub is_prone: bool,
    pub is_sliding: bool,
    pub is_ads: bool,
    pub is_airborne: bool,
    pub evt_jump_start: bool,
    pub evt_landed: bool,
}
