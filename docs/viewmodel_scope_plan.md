# Viewmodel Lighting + C7A2 Scope Plan

Source: user-provided implementation plan (saved verbatim for execution later).

## Issue 1: Weapon Models Look Flat (Activate & Enhance Viewmodel Shader)

### Root Cause
The custom viewmodel shader at `viewmodel_shader.rs` has a complete lighting pipeline (key light, fill light, Blinn-Phong specular, rim lighting, gamma-correct sRGB output) but is never used. `viewmodel.rs:220` calls `gl_use_default_material()` which runs macroquad's default shader: `gl_FragColor = vertex_color * texture2D(Texture, uv)` — zero lighting.

Additionally:

- `obj_loader.rs` loads static weapon meshes but only reads `base_color_texture` and `base_color_factor`. It discards `metallic_factor`, `roughness_factor`, normal maps, occlusion maps, and emissive maps.
- `animated_model.rs` around line ~647 stores `base_color_factor` as `(f[0] * 255.0) as u8` — raw linear values stuffed into u8 without `linear_to_srgb()` conversion. The static loader at `obj_loader.rs` does convert correctly. This causes animated models (like `fp_hands`) to appear either washed out or too dark.

### Step-by-step Implementation

#### Step 1: Store the viewmodel material on PlayingState

In `state.rs`:

- Add a field `pub viewmodel_material: Option<macroquad::material::Material>`
- Initialize it as `None` in `PlayingState::new()`

In `viewmodel.rs` `draw_3d_viewmodel()`:

At the top of the function, lazily create the material if it's `None`:

```rust
// In draw_viewmodel or draw_3d_viewmodel, before drawing:
if self.viewmodel_material.is_none() {
    self.viewmodel_material = Some(create_viewmodel_material());
}
```

(This requires `&mut self` — if the draw methods are `&self`, you'll need to either use `OnceCell/OnceLock`, or create the material during init in the asset loading phase instead.)

#### Step 2: Activate the shader before drawing

In `viewmodel.rs`, replace line 220:

```rust
// BEFORE (no lighting):
gl_use_default_material();

// AFTER (lit):
gl_use_material(&self.viewmodel_material.unwrap());
```

Do this for BOTH the weapon mesh draw AND the hands mesh draw (the `draw_hands` closure around line 233). Also apply to `draw_knife_viewmodel`.

After all draws are done (before `set_default_camera()`), call `gl_use_default_material()` to reset so other rendering isn't affected.

#### Step 3: Fix animated model gamma

In `animated_model.rs`, find the block around line 647 where `base_color_factor` is stored into vertex colors:

```rust
// CURRENT (wrong — linear stored as u8):
let mat_color = [
    (f[0] * 255.0) as u8,
    (f[1] * 255.0) as u8,
    (f[2] * 255.0) as u8,
    (f[3] * 255.0) as u8,
];

// FIX — use the same linear_to_srgb function that obj_loader.rs uses:
let mat_color = [
    linear_to_srgb(f[0]),
    linear_to_srgb(f[1]),
    linear_to_srgb(f[2]),
    (f[3] * 255.0) as u8,
];
```

The `linear_to_srgb` function already exists in `obj_loader.rs`. Either make it `pub` and import it, or copy the 3-line function into `animated_model.rs`:

```rust
fn linear_to_srgb(v: f32) -> u8 {
    let srgb = if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (srgb.clamp(0.0, 1.0) * 255.0) as u8
}
```

#### Step 4 (optional enhancement): Read metallic/roughness from GLTF

In `obj_loader.rs` `load_gltf_static_mesh_impl()`, after reading `base_color_factor`, also read:

```rust
let metallic = material.pbr_metallic_roughness().metallic_factor();   // 0.0-1.0
let roughness = material.pbr_metallic_roughness().roughness_factor(); // 0.0-1.0
```

The challenge is getting these per-primitive values to the shader. Since macroquad's `Mesh` doesn't have custom attribute slots, the pragmatic approach is:

- Store metallic/roughness as shader uniforms set before each `draw_mesh()` call
- Or bake a rough approximation: encode metallic into vertex color alpha (since viewmodel forces alpha=255 anyway, you could use a separate channel), though this is hacky

For the shader uniform approach, modify `viewmodel_shader.rs`:

```glsl
uniform float u_metallic;  // 0.0 = dielectric, 1.0 = metal
uniform float u_roughness; // 0.0 = mirror, 1.0 = matte

// Then in main():
float spec_power = mix(28.0, 128.0, 1.0 - u_roughness);
float spec_strength = mix(0.15, 0.45, u_metallic);
float spec = pow(max(dot(N, half_key), 0.0), spec_power) * spec_strength;
```

Set the uniforms via `material.set_uniform("u_metallic", metallic)` before each draw call. This requires storing the per-mesh metallic/roughness values alongside the mesh data — you'd need to extend the models `HashMap<String, Vec<Mesh>>` to also carry per-mesh PBR params, e.g. `models: HashMap<String, Vec<(Mesh, f32, f32)>>` or a wrapper struct.

## Issue 2: Scoped ADS for Colt C7A2 (3D Scope Alignment)

### Current Behavior
For `CrosshairStyle::Scope` weapons (snipers), `viewmodel.rs` lines ~26-31 hide the entire viewmodel when `ads_progress > 0.85`, and `crosshair.rs` draws a 2D scope overlay (vignette + crosshair lines + red dot). The `colt_c7a2.glb` model has a scope mesh built into it — we want the weapon to stay visible and slide up so the scope aligns with screen center.

### Step-by-step Implementation

#### Step 1: Add `CrosshairStyle::ScopeModel` variant

In `stats/mod.rs`:

```rust
pub enum CrosshairStyle {
    Standard,
    Scope,
    ScopeModel, // weapon has a built-in scope mesh, stays visible during ADS
}
```

#### Step 2: Add the weapon to `manifest.json`

In `manifest.json`, add:

```json
"colt_c7a2_model": "assets/models/weapons/Assault Rifles/colt_c7a2.glb"
```

#### Step 3: Create `WeaponId::ColtC7A2` and its stats

In `stats/mod.rs`:

- Add `ColtC7A2` to the `WeaponId` enum (under Assault Rifles)
- Add it to `WeaponId::ALL`
- Wire `WeaponId::ColtC7A2 => WeaponCategory::AssaultRifle` in `category()`
- Wire `WeaponId::ColtC7A2 => &assault_rifles::COLT_C7A2` in `stats()`

In `stats/assault_rifles.rs`, add a new const. Key fields that differ from a normal AR:

```rust
pub const COLT_C7A2: WeaponStats = WeaponStats {
    name: "Colt C7A2",
    crosshair_style: CrosshairStyle::ScopeModel,
    ads_zoom_fov: 30.0,  // significant zoom (normal ARs are 50-55)
    ads_time: 0.3,
    viewmodel_model: Some("colt_c7a2_model"),
    // The critical part — these offsets slide the scope to screen center:
    viewmodel_ads_pos_offset: (x, y, z),  // needs tuning in-game
    viewmodel_ads_rot_offset: (p, y, r),  // needs tuning in-game
    // ... rest similar to SCAR or M16
};
```

The `viewmodel_ads_pos_offset` and `viewmodel_ads_rot_offset` values control where the weapon slides to at full ADS. You need to tune these so the scope mesh aligns with the center of the screen. Start with something like `(0.35, 0.10, -0.05)` for position (moving it left+up+closer) and adjust from there. The existing dev mode viewmodel offset sliders will help with this.

#### Step 4: Keep viewmodel visible for `ScopeModel`

In `viewmodel.rs`, two early-return guards hide the viewmodel for scoped weapons:

Line ~26-31 (`draw_viewmodel`):

```rust
// BEFORE:
if matches!(stats.crosshair_style, CrosshairStyle::Scope)

// AFTER — only hide for the old 2D Scope style, not ScopeModel:
if matches!(stats.crosshair_style, CrosshairStyle::Scope)
```

This already doesn't match `ScopeModel`, so no change needed here.

Line ~80-85 (`draw_3d_viewmodel`):

```rust
// BEFORE:
if stats.crosshair_style == CrosshairStyle::Scope

// AFTER — same, already won't match ScopeModel:
if stats.crosshair_style == CrosshairStyle::Scope
```

Both guards already use exact `Scope` matching, so `ScopeModel` will pass through and the weapon stays visible. No code change needed for these guards — just confirm by reading the lines.

#### Step 5: Crosshair rendering for `ScopeModel`

In `crosshair.rs`, find the `CrosshairStyle::Scope` match arm (around line 382). Add a new arm:

```rust
CrosshairStyle::ScopeModel => {
    // At low ads_progress: draw normal crosshair (or nothing)
    // At high ads_progress: draw a subtle edge vignette
    if ads_t > 0.3 {
        let vignette_alpha = (((ads_t - 0.3) / 0.7) * 120.0) as u8;
        // Draw darkened edges — reuse draw_scope_vignette with a larger
        // inner radius so only the screen borders darken
        let scope_radius = screen_height().min(screen_width()) * 0.45;
        draw_scope_vignette(cx, cy, scope_radius, vignette_alpha);
    }
    // No crosshair lines or red dot — the 3D scope mesh IS the reticle
}
```

#### Step 6: Camera FOV

No changes needed in `camera.rs` — it already lerps the world camera FOV from `base_fov` to `ads_zoom_fov` based on `ads_progress`. Setting `ads_zoom_fov: 30.0` on the C7A2 will create the zoom automatically.

#### Step 7: Tuning the scope alignment

This is the most time-consuming part and must be done in-game:

1. Equip the C7A2 and enter dev mode
2. Use the ADS viewmodel offset sliders to adjust position/rotation until the scope's lens is centered on screen
3. Copy those values into the `COLT_C7A2` weapon stats const

The existing dev mode has `ads_pair_locked` / `ads_pos_offset` / `ads_rot_offset` sliders that let you adjust these values live. The values you land on get applied via the existing lerp in `draw_3d_viewmodel()` lines ~172-176.
