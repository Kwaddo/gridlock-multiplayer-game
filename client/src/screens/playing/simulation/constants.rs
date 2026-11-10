// ── Footstep constants ──
/// Maximum distance (world units) at which footsteps are audible
pub(super) const FOOTSTEP_MAX_DIST: f32 = 30.0;
/// Footstep intervals (seconds between steps)
pub(super) const FOOTSTEP_INTERVAL_WALK: f32 = 0.50;
pub(super) const FOOTSTEP_INTERVAL_SPRINT: f32 = 0.32;
pub(super) const FOOTSTEP_INTERVAL_CROUCH: f32 = 0.72;
/// Base volumes per movement type (before distance attenuation)
pub(super) const FOOTSTEP_VOLUME_WALK: f32 = 0.45;
pub(super) const FOOTSTEP_VOLUME_SPRINT: f32 = 0.75;
pub(super) const FOOTSTEP_VOLUME_CROUCH: f32 = 0.22;
/// Slide sound volume
pub(super) const SLIDE_SOUND_VOLUME: f32 = 0.6;
/// Height (world units) at which wall occlusion between listener and
/// source is tested: above props, below the top of maze walls.
pub(super) const SOUND_EAR_HEIGHT: f32 = 0.8;
/// Volume kept by a sound heard through a maze wall.
pub(super) const SOUND_OCCLUDED_VOLUME: f32 = 0.45;
/// Strongest stereo pan for world sounds (1 = fully one-sided).
pub(super) const SOUND_MAX_PAN: f32 = 0.8;

pub(super) const WORLD_PLAYER_RENDER_SCALE: f32 = 0.90;
pub(super) const PLAYER_MODEL_YAW_OFFSET: f32 = std::f32::consts::FRAC_PI_2;
pub(super) const HANDS_RELOAD_START_SEC: f32 = 0.0;
pub(super) const HANDS_RELOAD_END_SEC: f32 = 1.85;
pub(super) const PISTOL_RELOAD_START_SEC: f32 = 0.32;
pub(super) const PISTOL_RELOAD_END_SEC: f32 = 2.20;
#[allow(dead_code)]
pub(super) const PISTOL_EQUIP_START_SEC: f32 = 4.70;
#[allow(dead_code)]
pub(super) const PISTOL_EQUIP_END_SEC: f32 = 5.90;
pub(super) const SMG_RELOAD_START_SEC: f32 = 2.71;
pub(super) const SMG_RELOAD_END_SEC: f32 = 4.32;
#[allow(dead_code)]
pub(super) const SMG_EQUIP_START_SEC: f32 = 4.75;
#[allow(dead_code)]
pub(super) const SMG_EQUIP_END_SEC: f32 = 5.81;
pub(super) const SNIPER_RELOAD_START_SEC: f32 = 1.66;
pub(super) const SNIPER_RELOAD_END_SEC: f32 = 3.26;
#[allow(dead_code)]
pub(super) const SNIPER_EQUIP_START_SEC: f32 = 3.61;
#[allow(dead_code)]
pub(super) const SNIPER_EQUIP_END_SEC: f32 = 4.86;

// Minigun hand clip segments (minigun_animated.glb)
#[allow(dead_code)]
pub(super) const MINIGUN_IDLE_START_SEC: f32 = 0.00;
#[allow(dead_code)]
pub(super) const MINIGUN_IDLE_END_SEC: f32 = 0.00;
pub(super) const MINIGUN_SHOOT_LOOP_START_SEC: f32 = 0.01;
pub(super) const MINIGUN_SHOOT_LOOP_END_SEC: f32 = 1.17;
pub(super) const MINIGUN_RELOAD_START_SEC: f32 = 2.06;
pub(super) const MINIGUN_RELOAD_END_SEC: f32 = 4.29;
#[allow(dead_code)]
pub(super) const MINIGUN_EQUIP_START_SEC: f32 = 4.73;
#[allow(dead_code)]
pub(super) const MINIGUN_EQUIP_END_SEC: f32 = 5.30;

// Sawnoff hand clip segments (sawnoff_animated.glb)
#[allow(dead_code)]
pub(super) const SAWNOFF_IDLE_START_SEC: f32 = 0.00;
#[allow(dead_code)]
pub(super) const SAWNOFF_IDLE_END_SEC: f32 = 0.00;
pub(super) const SAWNOFF_SHOT_START_SEC: f32 = 0.01;
pub(super) const SAWNOFF_SHOT_END_SEC: f32 = 0.41;
pub(super) const SAWNOFF_RELOAD_START_SEC: f32 = 0.47;
pub(super) const SAWNOFF_RELOAD_END_SEC: f32 = 3.57;
#[allow(dead_code)]
pub(super) const SAWNOFF_EQUIP_START_SEC: f32 = 6.88;
#[allow(dead_code)]
pub(super) const SAWNOFF_EQUIP_END_SEC: f32 = 7.30;
// Carbine all-in-one clip segments (fps_animated_carbine.glb).
#[allow(dead_code)]
pub(super) const CARBINE_MELEE_HIT_START_SEC: f32 = 6.79;
#[allow(dead_code)]
pub(super) const CARBINE_MELEE_HIT_END_SEC: f32 = 7.40;
