//! Host uplink traffic bench: drives a `Server` the way the browser host does
//! (packets in, one `tick` per 60 Hz frame, outbox drained), with fake
//! clients that move, jump and shoot, and prints what each peer receives.
//!
//! Ignored by default; it runs in real time (the server reads the clock):
//!
//!   cargo test -p server --release -- --ignored --nocapture traffic_bench
//!
//! `BENCH_SECS` sets the measured match length (default 10 s).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use gridlock_server::{PeerId, Server};
use protocol::{ClientMsg, ServerMsg, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH};

const FRAME: Duration = Duration::from_nanos(16_666_667);
const HUMANS: u32 = 10;

/// One fake client: what it knows from the server, and where it walks.
struct Fake {
    peer: PeerId,
    id: u32,
    world: u8,
    origin: (f32, f32, f32),
    phase: f32,
}

#[derive(Default)]
struct TypeStats {
    packets: u64,
    bytes: u64,
}

/// What one measured window saw.
struct Window {
    label: &'static str,
    frames: u64,
    per_peer: HashMap<PeerId, TypeStats>,
    by_type: BTreeMap<String, TypeStats>,
    /// Frames between two `MSG_STATE`s to one peer, counted per gap length.
    state_gaps: BTreeMap<u64, u64>,
    last_state: HashMap<PeerId, u64>,
    tick_time: Duration,
    tick_max: Duration,
    work_time: Duration,
}

impl Window {
    fn new(label: &'static str) -> Self {
        Window {
            label,
            frames: 0,
            per_peer: HashMap::new(),
            by_type: BTreeMap::new(),
            state_gaps: BTreeMap::new(),
            last_state: HashMap::new(),
            tick_time: Duration::ZERO,
            tick_max: Duration::ZERO,
            work_time: Duration::ZERO,
        }
    }

    fn record(&mut self, frame: u64, peer: PeerId, bytes: &[u8]) {
        let p = self.per_peer.entry(peer).or_default();
        p.packets += 1;
        p.bytes += bytes.len() as u64;
        let name = ServerMsg::decode(bytes).map_or_else(|| format!("type{}", bytes[0]), |m| variant(&m));
        let t = self.by_type.entry(name).or_default();
        t.packets += 1;
        t.bytes += bytes.len() as u64;
        if bytes[0] == protocol::MSG_STATE {
            if let Some(prev) = self.last_state.insert(peer, frame) {
                *self.state_gaps.entry(frame - prev).or_default() += 1;
            }
        }
    }

    /// The window's report, built whole so parallel scenarios don't interleave.
    fn report(&self, scenario: &str) -> String {
        let mut out = String::new();
        let secs = self.frames as f64 * FRAME.as_secs_f64();
        let peers = self.per_peer.len().max(1) as f64;
        let total_pkts: u64 = self.per_peer.values().map(|s| s.packets).sum();
        let total_bytes: u64 = self.per_peer.values().map(|s| s.bytes).sum();
        let _ = writeln!(out, "== {scenario}: {} ({secs:.1} s, {} peers) ==", self.label, self.per_peer.len());
        let _ = writeln!(
            out,
            "  per peer: {:.1} pkt/s, {:.0} B/s | host uplink: {:.0} pkt/s, {:.0} B/s",
            total_pkts as f64 / secs / peers,
            total_bytes as f64 / secs / peers,
            total_pkts as f64 / secs,
            total_bytes as f64 / secs,
        );
        let mut types: Vec<_> = self.by_type.iter().collect();
        types.sort_by(|a, b| b.1.bytes.cmp(&a.1.bytes));
        for (name, s) in types {
            let _ = writeln!(
                out,
                "    {name:<16} {:>7.2} pkt/s/peer {:>8.0} B/s/peer",
                s.packets as f64 / secs / peers,
                s.bytes as f64 / secs / peers,
            );
        }
        let gaps: Vec<String> = self.state_gaps.iter().map(|(g, n)| format!("{g}f:{n}")).collect();
        let _ = writeln!(out, "  state gaps (frames:count): {}", gaps.join(" "));
        let _ = writeln!(
            out,
            "  tick: mean {:.1} us, max {:.1} us | frame work (packets+tick+drain): mean {:.1} us",
            self.tick_time.as_secs_f64() * 1e6 / self.frames.max(1) as f64,
            self.tick_max.as_secs_f64() * 1e6,
            self.work_time.as_secs_f64() * 1e6 / self.frames.max(1) as f64,
        );
        out
    }
}

/// `ServerMsg` variant name, for grouping.
fn variant(m: &ServerMsg) -> String {
    let s = format!("{m:?}");
    s.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("?").to_string()
}

struct Bench {
    server: Server,
    fakes: Vec<Fake>,
    /// Ids, teams and worlds of everyone (bots too), from the server.
    teams: HashMap<u32, u8>,
    alive: HashMap<u32, bool>,
    frame: u64,
    start: Instant,
    rng: u64,
}

impl Bench {
    fn new() -> Self {
        let mut b = Bench {
            server: Server::new(),
            fakes: Vec::new(),
            teams: HashMap::new(),
            alive: HashMap::new(),
            frame: 0,
            start: Instant::now(),
            rng: 0x2545_f491_4f6c_dd1d,
        };
        for i in 0..HUMANS {
            let peer = PeerId(i + 1);
            b.server.handle_packet(peer, &ClientMsg::join(&format!("human{i}")).encode());
            b.fakes.push(Fake { peer, id: 0, world: 0, origin: (0.0, 0.5, 0.0), phase: i as f32 });
        }
        b
    }

    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    /// Learn ids, teams, worlds and spawns from what a fake was sent.
    fn learn(&mut self, i: usize, msg: &ServerMsg) {
        match msg {
            ServerMsg::Welcome { id, .. } => self.fakes[i].id = *id,
            ServerMsg::PlayerInfo(infos) => {
                for info in infos {
                    self.teams.insert(info.id, info.team);
                }
            }
            ServerMsg::WorldChange { world, spawn, .. } => {
                self.fakes[i].world = *world;
                self.fakes[i].origin = *spawn;
            }
            ServerMsg::Respawn { position, .. } => self.fakes[i].origin = *position,
            ServerMsg::State { players, .. } => {
                for p in players {
                    self.alive.insert(p.id, p.alive);
                }
            }
            _ => {}
        }
    }

    /// One 60 Hz frame: client packets in, tick, outbox out. Sleeps to the
    /// frame's deadline first, so the server sees real 60 Hz timing.
    fn frame(&mut self, window: Option<&mut Window>, shoot_every: u64) {
        let deadline = self.start + FRAME * self.frame as u32;
        if let Some(wait) = deadline.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        let work_start = Instant::now();
        let t = self.frame as f32 * FRAME.as_secs_f32();
        for i in 0..self.fakes.len() {
            // the client sends position at 30 Hz: half the fakes each frame
            if (self.frame + i as u64) % 2 == 0 {
                let f = &self.fakes[i];
                let a = t * 0.8 + f.phase;
                let pos = (f.origin.0 + a.cos() * 1.5, f.origin.1, f.origin.2 + a.sin() * 1.5);
                let jump = self.rand() % 90 == 0;
                let msg = ClientMsg::Position {
                    position: pos,
                    yaw: a,
                    state_flags: Some(0),
                    event_flags: Some(if jump { protocol::EVT_JUMP_START } else { 0 }),
                    pitch: Some(0.0),
                };
                self.server.handle_packet(self.fakes[i].peer, &msg.encode());
            }
            // now and then, a lethal hit on someone of the other team
            if shoot_every > 0 && self.rand() % shoot_every == 0 && self.fakes[i].world == 1 {
                let me = (self.fakes[i].id, self.teams.get(&self.fakes[i].id).copied().unwrap_or(0));
                let mut targets: Vec<u32> = self
                    .teams
                    .iter()
                    .filter(|(id, team)| **id != me.0 && **team != me.1 && self.alive.get(id) == Some(&true))
                    .map(|(id, _)| *id)
                    .collect();
                targets.sort_unstable();
                if !targets.is_empty() {
                    let victim_id = targets[self.rand() as usize % targets.len()];
                    let hit = ClientMsg::Hit {
                        victim_id,
                        damage: 200.0,
                        weapon: Some("AK-47".into()),
                        backstab: Some(false),
                        headshot: Some(false),
                    };
                    self.server.handle_packet(self.fakes[i].peer, &hit.encode());
                }
            }
        }
        let tick_start = Instant::now();
        self.server.tick();
        let tick_time = tick_start.elapsed();
        let out = self.server.drain_outbox();
        let work_time = work_start.elapsed();
        let index: HashMap<PeerId, usize> = self.fakes.iter().enumerate().map(|(i, f)| (f.peer, i)).collect();
        let mut window = window;
        for (peer, bytes) in &out {
            if let Some(msg) = ServerMsg::decode(bytes) {
                if let Some(&i) = index.get(peer) {
                    self.learn(i, &msg);
                }
            }
            if let Some(w) = window.as_deref_mut() {
                w.record(self.frame, *peer, bytes);
            }
        }
        if let Some(w) = window {
            w.frames += 1;
            w.tick_time += tick_time;
            w.tick_max = w.tick_max.max(tick_time);
            w.work_time += work_time;
        }
        self.frame += 1;
    }

    fn run(&mut self, frames: u64, mut window: Option<&mut Window>, shoot_every: u64) {
        for _ in 0..frames {
            self.frame(window.as_deref_mut(), shoot_every);
        }
    }
}

fn bench_secs() -> f64 {
    std::env::var("BENCH_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(10.0)
}

fn frames(secs: f64) -> u64 {
    (secs / FRAME.as_secs_f64()).round() as u64
}

/// Join, measure the lobby, start the match, sit out warmup and the
/// post-teleport freeze, then measure the match.
fn scenario(name: &str, mode: u8, bots_per_team: Option<u8>) {
    let mut b = Bench::new();
    // settle: joins, team balance, first info sync
    b.run(frames(1.0), None, 0);

    let mut lobby = Window::new("lobby, 10 humans");
    b.run(frames(3.0), Some(&mut lobby), 0);

    let start = ClientMsg::StartGame {
        mode: Some(mode),
        duration: Some(0),
        autofill: bots_per_team.map(|n| (n, 0b11111)),
        blue_bots: bots_per_team,
    };
    let host = b.fakes[0].peer;
    b.server.handle_packet(host, &start.encode());
    // warmup countdown, then the 5 s freeze in the maze
    b.run(frames(5.0 + 5.5), None, 0);
    assert!(b.fakes.iter().all(|f| f.world == 1), "{name}: everyone should be in the maze");

    let mut maze = Window::new("match");
    // about one kill per human every 3 s
    b.run(frames(bench_secs()), Some(&mut maze), 180);

    print!("{}{}", lobby.report(name), maze.report(name));
}

#[test]
#[ignore]
fn traffic_bench_tdm() {
    scenario("TDM 10 humans", MODE_TEAM_DEATHMATCH, None);
}

#[test]
#[ignore]
fn traffic_bench_snd() {
    scenario("S&D 10 humans", MODE_SEARCH_AND_DESTROY, None);
}

#[test]
#[ignore]
fn traffic_bench_tdm_bots() {
    scenario("TDM 10 humans + 6 bots", MODE_TEAM_DEATHMATCH, Some(8));
}
