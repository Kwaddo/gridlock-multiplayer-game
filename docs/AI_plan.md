AI Bot System - Implementation Plan
Context
The game currently has no AI players. This plan introduces a server-side bot system that makes bots appear as real players to all connected clients. The immediate goal is Phase 1: a working shooting range in the lobby with bots that stand still, move, jump, and shoot back at players. This foundation will then scale into full match-integrated AI with 5 difficulty tiers and specialist archetypes.

Key decisions

Server-side bots (visible to all clients, broadcast via MSG_STATE)
Full physics port to server (not simplified)
5 tiers: Recruit / Easy / Normal / Hard / Nightmare
Tiers independent of weapon specialization
Fixed names per bot personality (video game references)
Tier-dependent FOV cone for vision
Simulated hitscan with reaction delay
Bots are full team members, kills fully counted
Auto-fill matches to 10 players
30Hz bot tick rate
Add glam to server for vector math
Replace existing dev mode Bot tab (Tab 5) with server-side bot controls
Shooting range as a separate walled area in the lobby
Phase 1: Foundation + Shooting Range Bot (DETAILED)
Phase 1 Split (No Scope Changes)
Phase 1A - Foundations
- Step 1: Add glam to server + math helpers
- Step 2: Port movement physics to server
- Step 3: Add is_bot field to Player

Phase 1B - Bot Core Scaffolding
- Step 4: Create bot module structure
- Step 5: BotManager (core orchestrator)
- Step 6: BotState and BotBehavior
- Step 16: Bot names

Phase 1C - Combat Pipeline Unification
- Step 8: Extract apply_damage from handle_hit

Phase 1D - Networking/Broadcast Hygiene
- Step 9: Skip network sends to bots
- Step 10: Skip bots in cleanup + handle bot respawns
- Step 11: Add is_bot flag to MSG_STATE protocol

Phase 1E - Runtime Integration + Controls
- Step 12: Integrate BotManager into server main loop
- Step 13: Bot control protocol messages
- Step 15: Replace dev mode Bot tab (Tab 5)

Phase 1F - Lobby Range Experience
- Step 14: Shooting range geometry in lobby
- Step 7: Bot shooting (simulated hitscan)
Step 1: Add glam to server + math helpers
File: server/Cargo.toml

Add glam = "0.29" to [dependencies]
New file: server/src/math.rs

Conversion helpers: to_vec3((f32,f32,f32)) -> Vec3 and from_vec3(Vec3) -> (f32,f32,f32)
Keeps existing Player.position as tuple, avoids refactoring all server code
Modify: server/src/main.rs

Add mod math;
Step 2: Port movement physics to server
New file: server/src/physics.rs

Port core physics from client/src/physics/movement.rs using glam::Vec3 instead of macroquad::prelude::Vec3 (identical API).


pub struct BotMovementState {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub height: f32,          // 1.6 default
}

pub struct PhysicsConfig {
    pub walk_speed: f32,      // 5.0
    pub sprint_speed: f32,    // 8.0
    pub crouch_speed: f32,    // 2.5
    pub gravity: f32,         // 20.0
    pub acceleration: f32,    // 10.0
    pub friction: f32,        // 8.0
    pub jump_strength: f32,   // 6.0
}
Core function:


pub fn simulate_bot_physics(
    state: &mut BotMovementState,
    position: &mut (f32, f32, f32),
    desired_direction: Vec3,  // normalized XZ move direction
    speed: f32,               // target speed (walk/sprint/crouch)
    want_jump: bool,
    dt: f32,
    config: &PhysicsConfig,
) -> (bool, bool)  // (evt_jump_start, evt_landed)
Physics to port from client MovementConfig::new() values:

Gravity: velocity.y -= gravity * dt
Ground collision: if position.1 <= 0.0, clamp, set grounded, zero Y velocity
Acceleration toward desired_direction * speed using acceleration * dt
Friction when grounded and no input: decelerate at friction * dt
Jump: if want_jump && is_grounded, set velocity.y = jump_strength
Wall collision is NOT needed in Phase 1 (lobby shooting range is open space). Will be added in Phase 2 for maze pathfinding.

Step 3: Add is_bot field to Player
Modify: server/src/player.rs

Add pub is_bot: bool to Player struct
Set is_bot: false in Player::new()
Step 4: Create bot module structure
New files:


server/src/bot/
  mod.rs          - pub mod manager; pub mod ai; pub mod shooting; pub mod names;
  manager.rs      - BotManager orchestrator
  ai.rs           - Per-bot tick logic (movement, targeting)
  shooting.rs     - Hitscan aim + fire logic
  names.rs        - Bot name pools per personality
Modify: server/src/main.rs

Add mod bot;
Step 5: BotManager (core orchestrator)
File: server/src/bot/manager.rs


/// Virtual SocketAddr for bots: 127.0.0.1:50000+index
/// Never collides with real clients (different IPs, OS ephemeral ports)
fn virtual_addr(index: u16) -> SocketAddr

pub struct BotManager {
    bot_addrs: Vec<SocketAddr>,                    // which addrs are bots
    bot_states: HashMap<SocketAddr, BotState>,     // AI state per bot
    last_tick: Instant,                            // 30Hz throttle
    tick_interval: Duration,                       // ~33ms
    next_bot_index: u16,                           // for virtual addr generation
}
Key methods:

new() -> Self
spawn_bot(players, name, position, world, team, behavior, shooting_config) -> SocketAddr
Creates Player with is_bot: true and virtual addr
Inserts into players HashMap
Creates corresponding BotState
despawn_bot(players, addr) - removes from both maps
despawn_all(players) - clear all bots
is_bot(addr) -> bool
tick(socket, players, match_end_pending, match_mode, dt) - runs at 30Hz:
Check elapsed time, skip if < 33ms since last tick
For each bot: call ai::tick_bot() for movement
For each bot: call shooting::try_shoot() for combat
If any hit detected, call shared apply_damage() function
Step 6: BotState and BotBehavior
File: server/src/bot/ai.rs


pub struct BotState {
    pub movement: BotMovementState,
    pub behavior: BotBehavior,
    pub shooting: BotShootingState,
    pub target_id: Option<u32>,       // current target player
    pub reaction_timer: f32,          // delay before shooting after spotting
}

pub enum BotBehavior {
    Static,
    Patrol {
        waypoints: Vec<(f32, f32, f32)>,
        current: usize,
        arrival_threshold: f32,
    },
    PatrolJump {
        waypoints: Vec<(f32, f32, f32)>,
        current: usize,
        jump_interval: f32,
        jump_timer: f32,
    },
}
tick_bot() function:

Determine desired movement direction from behavior (Static=zero, Patrol=toward waypoint)
Call simulate_bot_physics() with direction
Update Player entry: position, yaw (face movement dir), is_airborne, last_activity
Update yaw to face target player when shooting (not moving)
Step 7: Bot shooting (simulated hitscan)
File: server/src/bot/shooting.rs


pub struct BotShootingConfig {
    pub fire_interval: f32,     // seconds between shots
    pub angular_offset: f32,    // max random aim offset (radians)
    pub damage: f32,            // damage per hit
    pub range: f32,             // max engagement distance
    pub reaction_time: f32,     // delay before first shot after seeing target
}

pub struct BotShootingState {
    pub fire_cooldown: f32,
    pub has_target: bool,
    pub reaction_remaining: f32,
}
Shooting range level presets:

Level	fire_interval	angular_offset	damage	reaction_time
1 (Static, slow)	2.0s	0.15 rad (~8.5°)	8.0	1.5s
2 (Moving, faster)	1.0s	0.08 rad (~4.6°)	12.0	1.0s
3 (Moving+jumping, accurate)	0.5s	0.03 rad (~1.7°)	18.0	0.5s
try_shoot():

Decrement fire cooldown by dt
Find nearest alive non-bot player in same world within range
If no previous target, start reaction timer
If reaction timer elapsed and cooldown ready:
Compute direction from bot position to target position (aim at torso height ~1.2)
Add random angular offset (uniform in cone)
Check if offset ray would hit target (simple distance-based hit check for Phase 1)
Return Some((victim_id, damage)) on hit
Step 8: Extract apply_damage from handle_hit
Modify: server/src/server/handlers/other.rs

Extract the damage/kill/death logic from handle_hit (lines 165-294) into a reusable function:


pub fn apply_damage(
    socket: &UdpSocket,
    players: &mut HashMap<SocketAddr, Player>,
    attacker_addr: SocketAddr,
    victim_id: u32,
    damage: u32,
    weapon_name: &str,
    is_backstab: bool,
    match_end_pending: &mut Option<(Instant, u32, String)>,
    match_mode: u8,
)
This handles:

Health reduction
Death detection (health <= 0)
MSG_DEATH to victim (skip send_to if victim is bot)
MSG_KILL to attacker (skip send_to if attacker is bot)
Kill/death counter increments
Gun Game progression
MSG_KILL_FEED broadcast to all non-bot players
broadcast_state + broadcast_player_list
handle_hit then becomes a thin wrapper that parses the network payload and calls apply_damage.

Step 9: Skip network sends to bots
Modify: server/src/server/broadcast.rs

In ALL broadcast functions, skip socket.send_to() for players where p.is_bot == true:

broadcast_state() (line 122): if receiver.is_bot { continue; } before building per-receiver message
broadcast_player_list() (line 44): if p.is_bot { continue; } for the outer receiver loop
broadcast_match_timer() (line 76): if p.is_bot { continue; }
move_all_players_to_lobby() (line 107): if p.is_bot { continue; } before send_to
Also in server/src/server/handlers/other.rs:

handle_wall_broken: skip bot addrs in broadcast loop
handle_hit / apply_damage: skip send_to for bot victims/attackers
Kill feed broadcast: skip bot addrs
And in warmup/end countdown broadcasts in server/src/server/mod.rs (lines 194, 267): skip bots.

Bots still APPEAR in the state data sent to real players (they're in the HashMap), they just never receive packets themselves.

Step 10: Skip bots in cleanup + handle bot respawns
Modify: server/src/server/game.rs

cleanup_inactive_players() (line 222): Skip bots:


if player.is_bot { continue; }
process_auto_respawns() (line 128): Bots go through the same respawn logic but skip socket.send_to for the respawn message. Check player.is_bot before sending MSG_RESPAWN.

Step 11: Add is_bot flag to MSG_STATE protocol
Modify: server/src/protocols.rs


pub const FLAG_IS_BOT: u8 = 1 << 6;  // bit 6 of state_flags (bits 0-5 used)
Modify: server/src/server/broadcast.rs in broadcast_state():


if player.is_bot {
    state_flags |= FLAG_IS_BOT;
}
Modify: client/src/net/networking.rs - parse FLAG_IS_BOT from state_flags when processing MSG_STATE

Modify: client/src/types/player.rs - Add pub is_bot: bool to OtherPlayer

Client uses is_bot to show [BOT] prefix on nameplates and scoreboard.

### Client-side interpolation for bots (no extra work needed)

Bots receive the same interpolation treatment as real online players — no additional code is required. Here's how the existing pipeline handles it:

1. **Server broadcasts bot state at 30Hz** via `broadcast_state()` in `MSG_STATE`. Bot positions/yaw are included in the same packet as real players since bots live in the `players` HashMap.

2. **Client parses `MSG_STATE`** in `client/src/net/networking.rs` and calls `OtherPlayer::apply_snapshot(x, y, z, yaw)` for each player entry (bots included, since they're just entries with `is_bot: true`).

3. **`apply_snapshot()`** (`client/src/types/player.rs:285`) stores the latest server position as the **target** (`self.x/y/z/yaw`) and leaves `render_x/y/z/yaw` untouched. On the very first snapshot it snaps render to target. If the distance exceeds `SNAP_DISTANCE` (5.0 units) it also snaps (handles teleports/respawns).

4. **`smooth_render_state(delta)`** (`client/src/types/player.rs:316`) runs every frame via the `CombatEntity::update()` trait. It exponentially interpolates render position toward the target:
   - Position: `pos_alpha = 1.0 - exp(-22.0 * delta)` → ~22 units/sec smooth rate
   - Yaw: `yaw_alpha = 1.0 - exp(-18.0 * delta)` → ~18 rad/sec smooth rate via `lerp_angle()`

5. **All rendering** uses `render_position()` which returns `(render_x, render_y, render_z)`, so bots appear smoothly moving between their 30Hz server snapshots at the client's full frame rate.

**Why this works seamlessly for bots:** The client has no special codepath for "real player" vs "bot" in the rendering/interpolation layer. Every `OtherPlayer` goes through `apply_snapshot → smooth_render_state → render_position` regardless of origin. Since bots are broadcast in the same `MSG_STATE` packet, they automatically benefit from the same exponential smoothing, snap-on-teleport, and yaw interpolation that makes online players look fluid.

**No changes needed** to `OtherPlayer`, `apply_snapshot`, `smooth_render_state`, or any rendering code.

Step 12: Integrate BotManager into server main loop
Modify: server/src/server/mod.rs


use crate::bot::manager::BotManager;

pub fn run() -> std::io::Result<()> {
    // ... existing setup ...
    let mut bot_manager = BotManager::new();

    loop {
        // ... existing packet handling (lines 53-141) ...

        // NEW: Handle bot control messages
        // MSG_BOT_SPAWN and MSG_BOT_DESPAWN_ALL from dev mode

        // ... existing game loop logic (lines 143-274) ...

        // NEW: Bot AI tick (30Hz, internally throttled)
        bot_manager.tick(&socket, &mut players, &mut match_end_pending, match_mode);
    }
}
Step 13: Bot control protocol messages
Modify: server/src/protocols.rs


pub const MSG_BOT_SPAWN: u8 = 30;         // client -> server
pub const MSG_BOT_DESPAWN_ALL: u8 = 31;   // client -> server
Add matching constants to client protocol file.

MSG_BOT_SPAWN payload: [behavior:u8][level:u8]

behavior: 0=Static, 1=Patrol, 2=PatrolJump
level: 1-3 (shooting range difficulty)
Server handler creates bot at a predetermined shooting range position based on current bot count.

Step 14: Shooting range geometry in lobby
New file: server/src/lobby.rs

Define the shooting range as hardcoded AABB walls:


pub struct ShootingRangeConfig {
    pub walls: Vec<WallAABB>,
    pub bot_positions: Vec<(f32, f32, f32)>,  // spawn spots for range bots
    pub player_positions: Vec<(f32, f32, f32)>, // suggested player positions
}
Range layout: offset from lobby origin (e.g., x=15..25, z=-8..8)

Back wall, side walls, lane dividers (3 lanes)
Bot spawn positions at the far end of each lane
Patrol waypoints for moving bots (side-to-side within their lane)
Client-side rendering: Add range wall rendering to the lobby render pass. Simple cubes/quads matching the server-defined positions. This can be added to the existing lobby rendering code.

Step 15: Replace dev mode Bot tab (Tab 5)
Modify: client/src/screens/dev_mode/state.rs

Add fields for server-side bot control:
bot_spawn_level: u8 (1-3)
bot_spawn_behavior: u8 (0=Static, 1=Patrol, 2=PatrolJump)
Modify: client/src/screens/dev_mode/tabs.rs

Replace Tab 5 content with:
Level selector (1/2/3 dropdown)
"Spawn Bot" button -> sends MSG_BOT_SPAWN to server
"Clear All Bots" button -> sends MSG_BOT_DESPAWN_ALL
Active bot count display (count is_bot players from other_players list)
Step 16: Bot names
File: server/src/bot/names.rs

Static name pools per shooting range level:


// Level 1 (static targets) - named after easy/passive game characters
pub const LEVEL_1_NAMES: &[&str] = &["Goomba", "Slime", "Waddle Dee", "Magikarp"];

// Level 2 (moving targets) - slightly more threatening references
pub const LEVEL_2_NAMES: &[&str] = &["Koopa", "Creeper", "Bokoblin", "Grunt"];

// Level 3 (moving + jumping + accurate) - tougher references
pub const LEVEL_3_NAMES: &[&str] = &["Venom", "Slater", "Merlin", "Boomer"];
Names assigned in order of spawn, cycling through the pool.

Files Changed Summary (Phase 1)
Action	File
Modify	server/Cargo.toml (add glam)
New	server/src/math.rs
New	server/src/physics.rs
Modify	server/src/player.rs (add is_bot)
New	server/src/bot/mod.rs
New	server/src/bot/manager.rs
New	server/src/bot/ai.rs
New	server/src/bot/shooting.rs
New	server/src/bot/names.rs
New	server/src/lobby.rs
Modify	server/src/main.rs (add mod declarations)
Modify	server/src/protocols.rs (add bot messages + FLAG_IS_BOT)
Modify	server/src/server/mod.rs (integrate BotManager)
Modify	server/src/server/broadcast.rs (skip bot send_to, add is_bot flag)
Modify	server/src/server/game.rs (skip bots in cleanup/respawn sends)
Modify	server/src/server/handlers/other.rs (extract apply_damage, skip bot sends)
Modify	server/src/server/handlers/mod.rs (export new handlers)
Modify	client/src/types/player.rs (add is_bot field)
Modify	client/src/net/networking.rs (parse is_bot flag)
Modify	client/src/screens/dev_mode/state.rs (bot control fields)
Modify	client/src/screens/dev_mode/tabs.rs (replace Tab 5)
Modify	Client protocol file (add bot message constants)
New	Client lobby rendering additions (shooting range walls)
Phase 2: Maze Pathfinding (HIGH LEVEL)
Convert maze grid to navigation graph (nodes at cell centers, edges for open passages)
Store nav graph alongside MazeData
Port wall collision from client to server (AABB checks against maze walls)
BFS pathfinding for Recruit/Easy tiers
A* with heuristic for Normal+ tiers
Path recalculation frequency: every 2-5s depending on tier
Bots choose to move toward nearest enemy player using nav graph
Integrate wall collision into simulate_bot_physics()
Phase 3: Specialist Archetypes (HIGH LEVEL)
BotArchetype enum: Rusher, Sniper, Flanker, KnifeSneaker, AllRounder
Each archetype defines:
Preferred weapon (from existing WeaponId enum - 19 weapons available)
Preferred engagement range (sniper: far, rusher: close, knife: melee)
Behavior weights (aggression, patience, flanking tendency)
Fixed named bots per archetype:
Sniper: "Widowmaker", "Hanzo", "AWPer" etc.
Rusher: "Venom", "Reaper", "Doom" etc.
KnifeSneaker: "Spy", "Shadow", "Phantom" etc.
Weapon choice influences engagement distance and positioning decisions from Phase 2 pathfinding
Phase 4: Match Integration (HIGH LEVEL)
BotManager::auto_fill(players, target_count=10) - spawns bots to fill empty slots
Team balancing: distribute bots evenly across Red/Blue
Difficulty mixing based on average human player K/D ratio
Bots dynamically leave when human players join (maintain target count)
Bots participate in all game modes (TDM, Gun Game, Search & Destroy)
Dev mode: dropdown to select specific named bot to spawn
Configurable spawn rates per bot type (probability weights)
Phase 5: Advanced AI (HIGH LEVEL)
FOV cone perception (tier-dependent):
Recruit: 90° | Easy: 120° | Normal: 150° | Hard: 180° | Nightmare: 270°
Reaction time scaling:
Recruit: 2.0s | Easy: 1.2s | Normal: 0.6s | Hard: 0.3s | Nightmare: 0.1s
Aim accuracy (angular offset):
Recruit: 0.15 rad | Easy: 0.08 | Normal: 0.04 | Hard: 0.015 | Nightmare: 0.005
Pathfinding strategy:
Recruit: Random wandering | Easy: Direct path (BFS) | Normal: A* shortest | Hard: Flanking paths | Nightmare: Optimal + coordinated
Sound awareness: Gunshots create "noise events" that alert nearby bots
Coordinated pushes: Multiple bots on same team converge on single target
Cover usage: Higher tier bots peek from corners, don't stand in the open
Verification (Phase 1)
Server compiles: cd server && cargo build
Client compiles: cd client && cargo build
Start server: Run server, confirm "UDP server started on port 7878"
Connect client: Join lobby, verify no crashes
Dev mode bot spawn: Press F3, enter code, go to Bot tab, spawn a Level 1 bot
Bot appears: Verify bot appears as a player in the lobby with [BOT] tag
Bot is stationary: Level 1 bot stands still at shooting range position
Bot shoots: Bot should fire at you with poor aim after reaction delay
Bot takes damage: Shoot the bot, verify hit markers and damage numbers appear
Bot dies and respawns: Kill the bot, verify death animation, respawn after 3s
Kill feed: Verify bot kills/deaths appear in kill feed
Level 2 bot: Spawn Level 2, verify it moves side-to-side and shoots faster
Level 3 bot: Spawn Level 3, verify it moves, jumps, and aims more accurately
Clear all: Use "Clear All Bots" button, verify all bots disappear
Multiple clients: Connect 2 clients, verify both see the same bot

---

Progress Update (Do Not Edit Original Plan Above)
Date: 2026-02-25

Completed Today (remove these from pending)

Phase 1A
- Step 1: Add glam to server + math helpers
- Step 2: Port movement physics to server
- Step 3: Add is_bot field to Player

Phase 1B
- Step 4: Create bot module structure
- Step 5: BotManager core orchestrator (spawn/despawn/tick wired)
- Step 6: BotState and BotBehavior implemented and integrated
- Step 16: names file created (currently range flow is not using personality naming)

Phase 1C
- Step 8: apply_damage extracted and reused by bots

Phase 1D
- Step 9: Skip network sends to bots in broadcast/damage flows
- Step 10: Skip bots in cleanup + bot respawn handling updated
- Step 11: FLAG_IS_BOT added and client parse/display path wired

Phase 1E
- Step 12: BotManager integrated in server runtime loop
- Step 13: Bot control protocol messages wired (spawn/clear)
- Step 15: Dev mode bot tab path wired for server-side spawn controls

Phase 1F
- Step 14: Shooting range experience implemented in lobby (geometry, collision, start interaction)
- Step 7: Bot shooting implemented and iterated:
  - weapon damage/range sourced server-side from weapon stats
  - per-bot fire cadence/reaction/accuracy tuning
  - per-bot detection range support
  - randomized fire cadence (no synchronized volleys)
  - bot shot feedback packet path added (MSG_BOT_SHOT)

Additional Completed Today (not explicitly listed in original step text)
- Lobby health/death safety work:
  - player damage clamped in lobby (player does not hard-die there)
  - lobby regen delay + regen loop handling present
- Level design system for range:
  - Level 1: static shooters with mixed pistol/SMG loadouts and per-bot tuning
  - Level 2: moving shooters with mixed AR/shotgun loadouts and per-bot tuning
  - Level 3: timed 1-HP target challenge setup (non-shooting targets)
- Manual per-bot level definitions (explicit entries instead of generated map loops)
- Bot aiming behavior added (faces target when target exists)

Missing / Next Work (add these to active scope)

Priority Fixes (current blockers)
- Verify/fix remaining issues reported in playtests:
  - bots not dying consistently in shooting range
  - lobby player death still happening in some paths
  - bot tracer/muzzle visibility consistency
  - bot animation playback parity with normal players while moving/shooting
- Ensure no bots exist before range start in all code paths
- Confirm respawn-in-lobby behavior remains correct after latest combat changes

Gameplay Bots (separate from range bots) - Missing
- Create separate gameplay bot profile system (not tied to shooting range level presets)
- Difficulty system (independent of personality):
  - reaction time
  - angular offset
  - perception/FOV
  - repath frequency
  - strategy aggressiveness/flanking weights
- Maze pathfinding stack:
  - nav graph
  - BFS/A*
  - periodic repath
  - maze wall collision integration in server bot movement
- Personality bots for match gameplay:
  - fixed names per personality
  - archetype behavior weights
  - preferred weapons and ranges
- Runtime controls for match testing:
  - choose exact gameplay bot by name/profile
  - choose difficulty
  - choose team
  - spawn into real match context for testing
- Match auto-fill integration:
  - fill to target count
  - team balancing
  - dynamic leave when humans join

Verification Additions Needed
- Add explicit verification for:
  - no pre-start bot spawn in lobby range
  - Level 2 path variety (across-lane + non-linear patrols)
  - Level 3 timeout behavior
  - bot visual shot feedback for hit and miss cases
  - bot death/remove behavior in all range levels
