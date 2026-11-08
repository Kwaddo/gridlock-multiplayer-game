# Gridlock

A multiplayer first-person shooter written in Rust. Built on [macroquad](https://macroquad.rs/) for rendering on the client and a custom UDP-style protocol on a headless authoritative server. Two binaries, one workspace: `client/` and `server/`.

---

## Features

- **Authoritative server** — movement, hit detection, and weapon state computed server-side; the client renders and predicts.
- **Procedural maze arena** — generated server-side and streamed to clients on join.
- **Full weapon roster** — pistols (M1911, Deagle, Revolver, Suppressed, Taurus Raging), SMGs (MP5, P90, Honey Badger, Tommy Gun), shotguns (Benelli, SPAS), assault rifles (M4A1, AK, FAMAS, M16, SCAR, AS-VAL), snipers (Bolt Action, Hunting Rifle, Semi-Auto), and heavies (Grenade Launcher, Minigun, RPG).
- **Per-weapon stats and recoil** — server-validated stats table (`server/src/weapons_stats.rs`) with client-side recoil/spread (`client/src/weapons/recoil.rs`).
- **Animated FP viewmodels** — separate rigged hand rigs per weapon class (carbine, pistol, SMG, sniper, minigun, sawn-off).
- **3rd-person player animations** — walk/sprint/crouch/prone/jump/slide/death sets in `assets/models/player/`.
- **Crosshair presets** — JSON-driven crosshair customization (`client/crosshair_presets_shadow.json`).
- **Bots** — server-side AI players (`server/src/bot/`).
- **Bombsite / objective logic** — see `server/src/server/game.rs`.
- **Pause menu, dev mode, lobby inventory, login** — modular screen system in `client/src/screens/`.
- **Chat and match control** — text chat (`T` or `Enter`, rebindable) with join/leave notices; from the pause menu the host can end a match for everyone and anyone can leave a match, both returning to the lobby without disconnecting.
- **Spatial audio** — `rodio` on the client for footsteps, gunfire, slides.

---

## Repository Layout

```
multiplayer-fps/
├── Cargo.toml             # Workspace root
├── client/                # macroquad-based game client
│   ├── src/
│   │   ├── app/           # App bootstrap / main loop wiring
│   │   ├── core/          # Shared client primitives
│   │   ├── net/           # Networking (client side)
│   │   ├── physics/       # Client-side prediction / collision
│   │   ├── screens/       # Login, lobby, playing, pause, dev mode
│   │   ├── settings/      # User settings
│   │   ├── types/         # Shared types
│   │   ├── ui/            # HUD, menus
│   │   ├── weapons/       # Combat, recoil, crosshair, effects, loadout, stats
│   │   ├── world/         # World/maze rendering
│   │   ├── lib.rs
│   │   └── main.rs
│   ├── assets/            # Models, sounds, textures, sprites
│   └── crosshair_presets_shadow.json
├── server/                # Authoritative game server
│   └── src/
│       ├── bot/           # Bot AI
│       ├── maze/          # Procedural maze generation
│       ├── server/        # game loop, broadcast, handlers
│       ├── log.rs
│       ├── math.rs
│       ├── physics.rs
│       ├── player.rs
│       ├── protocols.rs   # Wire protocol
│       ├── weapons_stats.rs
│       └── main.rs
├── manifest.json          # Asset path manifest (loaded by client)
├── docs/                  # Design notes (game loop, viewmodel, tracers)
├── build_linux.sh         # Release build + tarball
├── build_mac.sh
├── build_windows.sh
├── install_linux.sh       # User-local install (~/.local/share/gridlock)
└── installer.nsi          # Windows NSIS installer
```

---

## Requirements

- Rust (stable, edition 2021)
- Linux: standard X11/Wayland + OpenGL drivers
- macOS: Xcode command-line tools
- Windows: MSVC toolchain (for native build) or use the provided NSIS installer

Dependencies (managed by Cargo):

- `macroquad 0.4` — windowing, input, rendering (client)
- `glam 0.29` — math (shared)
- `gltf 1.4` — GLB model loading (client)
- `rodio 0.20` — audio (client)
- `serde` / `serde_json` — config + protocol
- `rand 0.8` — RNG (both)

---

## Building

From the workspace root:

```bash
# Debug build of both binaries
cargo build

# Release build
cargo build --release

# Run individually
cargo run -p server --release
cargo run -p client --release
```

Or use the platform scripts (produce a packaged `dist/` tarball/zip):

```bash
./build_linux.sh        # → dist/Gridlock-Linux.tar.gz
./build_mac.sh          # → dist/mac/
./build_windows.sh      # → dist/windows/
```

Release profile is tuned for size: `opt-level = "z"`, LTO on, single codegen unit, stripped, panic = abort.

### File size limit

No source file (`.rs`, `.js`, `.mjs`, `.sh`) may be longer than 500 lines. Check with:

```bash
./scripts/check_line_limit.sh
```

When a file grows past the limit, split it into modules by responsibility, with its constants in their own module (for example `foo/constants.rs`), instead of raising the limit. The only exception is `registry/dcs/index.js`, which is deployed as a single inline file.

---

## Installing (Linux)

```bash
tar -xzf dist/Gridlock-Linux.tar.gz
cd linux
./install.sh
```

Installs to `~/.local/share/gridlock`, symlinks `gridlock` into `~/.local/bin`, and adds a `.desktop` entry. Uninstall via `~/.local/share/gridlock/uninstall.sh`.

---

## Running

1. Start the server:
   ```bash
   ./server          # or: cargo run -p server --release
   ```
2. Launch the client(s):
   ```bash
   gridlock          # or: cargo run -p client --release
   ```
3. Log in from the client screen, pick a loadout in the lobby, join the match.

**Browser, offline or on your network:** `./play_local.sh` builds the browser
version and serves it with a local registry (Node.js 18+). Open the address it
prints on any machine on the network, then host or join. See `web/README.md`.

---

## Assets

The client loads all art/audio paths from `manifest.json` at startup. Keys are grouped by scope (`global`, `lobby`, `maze`). Asset categories:

- **Hands / viewmodels** — `assets/models/hand/*.glb`
- **Weapons** — `assets/models/weapons/<Class>/*.glb`
- **Player animations** — `assets/models/player/*.glb`
- **Sounds** — `assets/sounds/*.ogg`
- **Sprites / icons** — `assets/images/weapons/`
- **Crosshairs** — `assets/images/weapons/kenney_crosshair-pack/`

---

## Networking

- Server is authoritative. Clients send inputs; server returns world state.
- Wire format defined in `server/src/protocols.rs`.
- Broadcast loop in `server/src/server/broadcast.rs`; per-client handlers in `server/src/server/handlers/`.

---

## Docs

Design / implementation notes live in `docs/`:

- `game-loop-phases.md` — tick phases, ordering
- `billboard_tracers_explained.txt` — tracer rendering
- `viewmodel_scope_plan.md` — ADS / scoping
- `AI_plan.md` — bot behavior plan

---

## License

No license file is currently present. Treat as all-rights-reserved until one is added.
