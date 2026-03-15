# Agent Instructions for Kasy

## Repository Overview

**Kasy** is a 3-D isometric game written in **Rust** using the **Bevy 0.15** game engine.
The player controls a bee that flies over floating islands, avoids flowers, crosses lava at high
speed, and must find the single red flower to win.  The game targets Linux desktop (native) and
browsers (WebAssembly via WebGPU / WebGL 2).

```
Kasy/
├── src/
│   ├── main.rs        – app entry-point, plugin registration, global lighting
│   ├── camera.rs      – 3-D follow camera with bloom post-processing
│   ├── player.rs      – bee entity, keyboard movement, lava/respawn logic, HUD
│   ├── flower.rs      – regular/red flowers, win/collision detection
│   ├── world.rs       – procedural island generation, tile meshes, decorations, LavaTiles
│   ├── iso.rs         – isometric coordinate helpers (grid ↔ world ↔ plane)
│   ├── enemy.rs       – wandering enemy entities (not yet wired into main.rs)
│   └── projectile.rs  – water-gun projectile events/systems (not yet wired into main.rs)
├── .github/
│   ├── agents.md      – this file
│   └── workflows/
│       └── deploy.yml – GitHub Actions: build WASM → deploy to GitHub Pages
├── Cargo.toml         – workspace manifest; Bevy 0.15 feature flags
├── index.html         – WASM host page (canvas #bevy)
└── README.md          – end-user documentation
```

---

## Working with the Repository

### Prerequisites

| Tool | Install command |
|---|---|
| Rust stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| WASM target | `rustup target add wasm32-unknown-unknown` |
| wasm-bindgen | `cargo install wasm-bindgen-cli --locked` |
| xvfb (Linux, for headless runs) | `sudo apt-get install -y xvfb` |
| scrot (Linux screenshots) | `sudo apt-get install -y scrot` |

### Key commands

```bash
# Fast compile check (no linker – use during iteration)
cargo check

# Run all tests (compiles the entire binary; 0 unit tests today)
cargo test

# Run on desktop
cargo run

# Build WASM release bundle
cargo build --profile wasm-release --target wasm32-unknown-unknown

# Generate JS glue + copy assets
mkdir -p dist
wasm-bindgen --out-dir dist --target web \
  target/wasm32-unknown-unknown/wasm-release/kasy.wasm
cp index.html dist/

# Serve locally
python3 -m http.server --directory dist   # open http://localhost:8000
```

### Controls (desktop)

| Input | Action |
|---|---|
| **W A S D** | Move (strafing) |
| **Mouse** | Aim / rotate |
| **Left Click** | Fire water gun |
| **Space** | Thrust (bee flight) / restart after win |
| **Arrow keys** | Steer (turn bee) |

---

## Maintaining README.md and agents.md in Pull Requests

Every PR that changes **user-visible behaviour** (new mechanics, controls, objectives, platforms,
build steps) **must** update `README.md` to reflect the change.

Every PR that changes **how agents should work with the codebase** (new modules, altered build
process, new coding conventions, new external tools) **must** update this file (`agents.md`).

Checklist before merging:

- [ ] Does the PR add or remove a game mechanic? → update **Controls** or **How to play** in
  `README.md`.
- [ ] Does the PR add a new Rust module? → add it to the **directory tree** in both `README.md`
  (if user-relevant) and in this file.
- [ ] Does the PR change the build or deployment process? → update the **Development** section of
  `README.md` and the **Key commands** section of this file.
- [ ] Does the PR add or remove dependencies? → document the new prereq in both files as
  appropriate.
- [ ] Does the PR introduce a new coding pattern not yet documented here? → add it to the
  **Bevy Development Guide** section below.

---

## Taking Screenshots for Pull Requests

Including a screenshot (or short screen-recording GIF) in every PR that touches visual output
lets reviewers quickly verify the change looks correct.

### Install dependencies (Linux)

```bash
sudo apt-get update
sudo apt-get install -y \
  xvfb \
  scrot \
  openbox \
  mesa-vulkan-drivers \
  libxkbcommon-x11-0
```

> **GPU-less CI environments** (no real GPU): Mesa's `mesa-vulkan-drivers` provides the
> **lavapipe** software Vulkan renderer.  Without it Bevy will panic with
> `Unable to find a GPU!`.  `libxkbcommon-x11-0` is also required for the X11 keyboard
> driver that winit loads at runtime.  `openbox` is a lightweight window manager that
> prevents winit from closing the window immediately on a bare Xvfb display.

### Capture a screenshot from a headless display

```bash
# 1. Start a virtual display and a lightweight window manager
Xvfb :99 -screen 0 1280x720x24 &
export DISPLAY=:99
openbox &
sleep 1   # let openbox register

# 2. Build (skip if already compiled)
cargo build

# 3. Run the game in the background.
#    VK_ICD_FILENAMES forces lavapipe (software Vulkan) on GPU-less machines;
#    omit this line when a real GPU is available.
mkdir -p screenshots
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  ./target/debug/kasy &
GAME_PID=$!

# 4. Wait for the world to render (software renderer is slow), then capture.
#    ~20 s is enough for the procedural world to finish painting.
sleep 20
scrot screenshots/gameplay.png

# 5. Tidy up
kill $GAME_PID
unset DISPLAY
```

> **Tip:** On a machine with a real GPU you can skip `VK_ICD_FILENAMES`, drop the
> `openbox` step, and reduce the sleep to 8 s.

### Attach to the PR

1. Copy the `screenshots/` folder into your branch.
2. Reference the image in your PR description:
   ```markdown
   ![Gameplay screenshot](screenshots/gameplay.png)
   ```
3. Commit the screenshot file alongside your code changes.

> **Note:** The `screenshots/` folder is **not** gitignored by default.  Add
> `screenshots/` to `.gitignore` if you prefer to attach images as GitHub PR
> comments rather than committing them.

---

## Bevy 0.15 Development Guide

This section captures the conventions and patterns used throughout Kasy.  Read it before
writing or reviewing Bevy code in this repository.

### Core architecture: ECS

Bevy uses an **Entity–Component–System** (ECS) architecture.

| Concept | What it is | How it appears in Kasy |
|---|---|---|
| **Entity** | A unique ID – the "thing" in the world | bee, tile, flower, camera, UI node |
| **Component** | Data attached to an entity | `Player`, `BeeVelocity`, `Flower`, `LavaTiles` |
| **System** | A function that runs every frame (or on startup) | `player_movement`, `setup_world` |
| **Resource** | Global singleton data | `RespawnState`, `LavaTiles`, `Time`, `Assets<Mesh>` |
| **Plugin** | A group of systems + resources registered together | `WorldPlugin`, `PlayerPlugin` |

### Plugin pattern

Every module exposes exactly one `pub struct XxxPlugin` that implements `Plugin`:

```rust
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MyResource>()
           .add_systems(Startup, setup_my_thing)
           .add_systems(Update, (update_a, update_b.after(update_a)));
    }
}
```

Register the plugin in `main.rs`:

```rust
.add_plugins((
    world::WorldPlugin,
    camera::CameraPlugin,
    player::PlayerPlugin,
    flower::FlowerPlugin,
    my_module::MyPlugin,   // ← add here
))
```

### Systems

```rust
// Startup system – runs once after all startup systems have finished
fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) { ... }

// Update system – runs every frame
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut BeeVelocity), With<Player>>,
) { ... }
```

**System ordering within a set:**

```rust
.add_systems(Update, (
    handle_respawn,
    player_movement.after(handle_respawn),
    check_lava.after(player_movement),
))
```

### Queries

```rust
// Read-only, single entity
let Ok(player_tf) = player_query.get_single() else { return; };

// Mutable, single entity
let Ok((mut transform, mut vel)) = query.get_single_mut() else { return; };

// Iterate over many entities
for (entity, transform) in &query { ... }
for (entity, mut transform) in &mut query { ... }

// Filter by component presence
Query<&Transform, With<Player>>
Query<&Transform, Without<Player>>
Query<&Transform, (With<Flower>, Without<RedFlower>)>
```

### Spawning entities

```rust
// Simple entity
commands.spawn((
    Player,                   // marker component
    BeeVelocity(Vec2::ZERO),
    Mesh3d(body_mesh),
    MeshMaterial3d(body_material),
    Transform::from_xyz(x, y, z),
));

// Entity with children
commands.spawn((...))
    .with_children(|parent| {
        parent.spawn((...));
    });

// Despawn (and all children)
commands.entity(entity_id).despawn_recursive();
```

### Transforms (3-D)

The world uses a **Y-up, right-handed** coordinate system with an isometric view from above.

```
X → right (east)
Y → up   (altitude)
Z → towards viewer (south)
```

The isometric projection maps grid `(gx, gy)` → world `(x, z)` – the Y component is the
height above the tile surface (see `src/iso.rs`).

```rust
Transform::from_xyz(x, y, z)
Transform::from_translation(vec3).with_scale(Vec3::splat(2.0))
Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y)
transform.rotate_y(angle_radians);
```

### Materials and meshes (3-D PBR)

```rust
// Primitive meshes
meshes.add(Cuboid::new(w, h, d))
meshes.add(Sphere::new(r).mesh().ico(3).unwrap())
meshes.add(Cylinder::new(radius, height))

// PBR material
materials.add(StandardMaterial {
    base_color: Color::srgb(r, g, b),
    emissive: LinearRgba::rgb(r, g, b),   // self-glow (HDR values > 1.0 work)
    perceptual_roughness: 0.9,
    alpha_mode: AlphaMode::Blend,          // for transparency
    unlit: true,                           // ignore lighting
    ..default()
})

// Attach to entity
Mesh3d(mesh_handle),
MeshMaterial3d(material_handle),
```

### Lights

```rust
// Ambient (global)
app.insert_resource(AmbientLight { color: Color::srgb(0.05, 0.05, 0.08), brightness: 80.0, ..default() });

// Directional (sun-like)
commands.spawn((
    DirectionalLight { illuminance: 250.0, shadows_enabled: true, ..default() },
    Transform::from_xyz(-180.0, 260.0, 120.0).looking_at(Vec3::ZERO, Vec3::Y),
));

// Point (omni)
commands.spawn((
    PointLight { intensity: 3_500_000.0, range: 125.0, color: Color::srgb(1.0, 0.85, 0.65), ..default() },
    Transform::from_xyz(x, y, z),
));

// Spot
commands.spawn((
    SpotLight { intensity: 1_000_000.0, inner_angle: 0.55, outer_angle: 1.05, ..default() },
    Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
));
```

### Resources

```rust
// Define
#[derive(Resource, Default)]
pub struct MyResource { pub value: f32 }

// Register (in plugin)
app.init_resource::<MyResource>()
// or with an initial value:
commands.insert_resource(MyResource { value: 42.0 });

// Use in a system
fn my_system(mut res: ResMut<MyResource>) { res.value += 1.0; }

// Optional resource (may not exist yet)
fn my_system(res: Option<Res<WorldTiles>>) {
    let Some(tiles) = res else { return; };
}
```

### Events

```rust
// Define
#[derive(Event)]
pub struct SpawnProjectile { pub position: Vec2, pub direction: Vec2 }

// Register
app.add_event::<SpawnProjectile>()

// Send
fn fire(mut writer: EventWriter<SpawnProjectile>) {
    writer.send(SpawnProjectile { position: pos, direction: dir });
}

// Receive
fn handle(mut reader: EventReader<SpawnProjectile>) {
    for event in reader.read() { ... }
}
```

### UI

Bevy UI uses a CSS-like flexbox model.  All UI nodes are 2-D and rendered in screen space.

```rust
commands.spawn((
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(8.0),
        ..default()
    },
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
    GlobalZIndex(999),
    Visibility::Hidden,   // show/hide by mutating this component
));

// Text
commands.spawn((
    Text::new("Hello World"),
    TextFont { font_size: 24.0, ..default() },
    TextColor(Color::srgb(1.0, 1.0, 1.0)),
));

// Update text content
*text = Text::new(format!("Speed: {speed}"));
```

### Time and delta-time

Always multiply velocities and other per-frame values by `time.delta_secs()`:

```rust
fn my_system(time: Res<Time>, mut query: Query<&mut Transform>) {
    let dt = time.delta_secs();
    for mut tf in &mut query {
        tf.translation.x += SPEED * dt;
    }
}
```

### WASM / cross-platform guards

```rust
#[cfg(target_arch = "wasm32")]
canvas: Some("#bevy".to_string()),

#[cfg(not(target_arch = "wasm32"))]
// desktop-only code
```

Keep WASM-specific code in conditional blocks.  The `getrandom` feature flags in `Cargo.toml`
must be kept in sync if you add new randomness dependencies.

### Isometric coordinate system (Kasy-specific)

| Function | Direction |
|---|---|
| `iso::grid_to_world(gx, gy) → Vec2` | Grid → world (x, z) |
| `iso::world_to_grid(Vec2) → (i32, i32)` | World (x, z) → nearest grid cell |
| `iso::world_to_plane(Vec3) → Vec2` | Extract (x, z) from a 3-D translation |

`TILE_WIDTH = 64`, `TILE_HEIGHT = 32` define the diamond footprint of each tile.

### Adding a new module / feature

1. Create `src/my_feature.rs`.
2. Add `mod my_feature;` in `src/main.rs`.
3. Implement `pub struct MyFeaturePlugin` → `impl Plugin`.
4. Register it in `main.rs` inside `.add_plugins((...))`.
5. Update this file and `README.md` as needed.

### Common pitfalls

| Pitfall | Fix |
|---|---|
| Query borrowing the same component mutably twice | Split into two separate queries |
| Forgetting `..default()` in struct literals | Bevy structs have many optional fields |
| `get_single()` panics when more than one entity matches | Use `get_single` + handle `Err`, or ensure the query is specific enough |
| Forgetting `.after()` when system order matters | Use `.after(other_system)` in `add_systems` |
| Missing `GlobalTransform` on a spawned entity | Bevy warns; add `GlobalTransform::default()` when not using a bundle |
| Children not visible | Ensure parent has `Visibility`, `InheritedVisibility`, `ViewVisibility` |
| WASM build fails on `getrandom` | Check `Cargo.toml` for the `wasm_js` / `js` feature flags |
