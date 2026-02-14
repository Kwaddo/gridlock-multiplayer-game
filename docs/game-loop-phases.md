## Game Loop Phases & Pause System

The main loop in `client/src/main.rs` runs every frame. Each frame goes through
up to 5 phases, but which phases run depends on the current state.


### The 5 Phases

1. **Network Receive** (`sync_world`)
   - Reads incoming UDP packets from the server
   - Updates other player positions, world changes, maze data, asset list
   - Runs EVERY frame regardless of state (Playing or Paused)

2. **Input** (`process_local_input`)
   - Reads mouse movement, keyboard, weapon input
   - Fires weapons, handles recoil/bloom
   - Returns a state transition if ESC is pressed (Playing -> Paused)
   - ONLY runs when Playing

3. **Simulation** (`simulate`)
   - Physics: gravity, movement, collision
   - Weapon cooldowns, reload timers, recoil recovery
   - Target dummy updates, hit marker timers
   - Runs in BOTH Playing and Paused
   - Why: so physics settles (player doesn't float mid-air when pausing)

4. **Network Send** (`send_network`)
   - Sends local player position to server via UDP
   - Sends pending world change requests
   - ONLY runs when Playing
   - Why: no input = no movement = nothing new to send

5. **Render** (`render`)
   - Draws the 3D world, other players, HUD, crosshair
   - Runs in BOTH Playing and Paused
   - Why: you still see the game behind the pause menu


### What runs in each state

```
Phase              | Playing | Paused
-------------------|---------|--------
1. Network Receive |   YES   |  YES
2. Input           |   YES   |  NO
3. Simulation      |   YES   |  YES
4. Network Send    |   YES   |  NO
5. Render          |   YES   |  YES
```

Paused skips Input and Network Send. Everything else keeps running.
The pause menu is drawn ON TOP of the rendered game (after phase 5).


### How pausing works

**Playing -> Paused** (ESC pressed):
- `process_local_input` detects ESC, returns `GameState::Paused`
- Cursor is released (`set_cursor_grab(false)`, `show_mouse(true)`)
- `clear_input()` zeroes out movement so the player stops moving
- The loop `continue`s — skipping simulate/send/render for THIS frame only
- Next frame enters the Paused branch

**Paused -> Playing** (Resume clicked):
- `draw_pause_menu` returns `GameState::Playing`
- Cursor is grabbed again
- `last_mouse_pos` is reset so the camera doesn't jump
- Next frame enters the Playing branch


### Why simulation runs while paused

If you paused mid-jump, the player would freeze in the air without simulation.
Running physics while paused lets gravity finish pulling the player down and
velocity decay to zero. Since there's no input, the player just comes to rest.


### The Settings detour

Paused -> Settings -> Paused is a separate state. Settings only draws its own
UI and updates config values. When you close settings, it goes back to Paused
(not Playing), so the pause menu reappears.

```
Playing --ESC--> Paused --Settings btn--> Settings --back--> Paused --Resume--> Playing
```
