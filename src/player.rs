use bevy::prelude::*;

use crate::world::LavaTiles;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RespawnState>()
            .add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (
                    handle_respawn,
                    player_movement.after(handle_respawn),
                    check_lava.after(player_movement),
                    update_speed_hud.after(player_movement),
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Components / resources / constants
// ---------------------------------------------------------------------------

/// Marker component that identifies the player entity.
#[derive(Component)]
pub struct Player;

/// Current velocity of the bee in world-space units per second.
#[derive(Component)]
pub struct BeeVelocity(pub Vec2);

/// Marker for the full-screen black overlay used for the fade-to-black effect.
#[derive(Component)]
struct FadeOverlay;

/// Marker for the speed readout text in the HUD.
#[derive(Component)]
struct SpeedHud;

/// Which phase of the respawn sequence we are in.
#[derive(Default, PartialEq)]
pub enum RespawnMode {
    /// Normal gameplay.
    #[default]
    Normal,
    /// Fading the screen to black before teleporting.
    FadingOut,
    /// Screen is fully black; fading back in at the spawn point.
    FadingIn,
}

/// Tracks respawn state.  Stored as a resource so all systems share it.
#[derive(Resource, Default)]
pub struct RespawnState {
    pub mode:  RespawnMode,
    /// Current overlay alpha (0 = transparent, 1 = fully black).
    pub alpha: f32,
}

/// Radius of the bee body sprite in pixels (used for collision checks).
pub const PLAYER_RADIUS: f32 = 10.0;

/// How fast the bee can turn (radians per second).
const TURN_SPEED: f32 = 3.0;
/// Thrust acceleration when Space is held (world units per second²).
const THRUST_ACCEL: f32 = 320.0;
/// Maximum flight speed (world units per second).
const MAX_SPEED: f32 = 380.0;
/// Linear drag coefficient — velocity decays as `vel *= (1 − DRAG·dt).max(0)`.
const LINEAR_DRAG: f32 = 1.5;
/// Minimum speed required to safely cross lava tiles.
const LAVA_MIN_SPEED: f32 = 120.0;
/// Speed of the fade-to-black transition (alpha units per second).
const FADE_SPEED: f32 = 2.5;
/// World-space spawn position.
pub const PLAYER_SPAWN: Vec2 = Vec2::new(0.0, 8.0);

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // --- Bee body: yellow ellipse ---
    let body_mesh     = meshes.add(Ellipse::new(PLAYER_RADIUS, PLAYER_RADIUS * 1.5));
    let body_material = materials.add(Color::srgb(1.0, 0.85, 0.05));

    // --- Bee stripes: dark thin rectangles ---
    let stripe_mesh     = meshes.add(Rectangle::new(PLAYER_RADIUS * 2.2, 4.0));
    let stripe_material = materials.add(Color::srgb(0.08, 0.08, 0.08));

    // --- Bee stinger: small dark triangle approximated by a thin rectangle ---
    let stinger_mesh     = meshes.add(Rectangle::new(3.0, 5.0));
    let stinger_material = materials.add(Color::srgb(0.12, 0.08, 0.04));

    // --- Wings: translucent pale-blue circles on each side ---
    let wing_mesh     = meshes.add(Circle::new(PLAYER_RADIUS * 1.3));
    let wing_material = materials.add(Color::srgba(0.80, 0.92, 1.00, 0.55));

    commands
        .spawn((
            Player,
            BeeVelocity(Vec2::ZERO),
            Mesh2d(body_mesh),
            MeshMaterial2d(body_material),
            // z = 10 keeps the bee above all tile geometry.
            Transform::from_xyz(PLAYER_SPAWN.x, PLAYER_SPAWN.y, 10.0),
        ))
        .with_children(|parent| {
            // Two body stripes
            parent.spawn((
                Mesh2d(stripe_mesh.clone()),
                MeshMaterial2d(stripe_material.clone()),
                Transform::from_xyz(0.0, 5.0, 0.1),
            ));
            parent.spawn((
                Mesh2d(stripe_mesh),
                MeshMaterial2d(stripe_material),
                Transform::from_xyz(0.0, -3.0, 0.1),
            ));

            // Stinger at the rear (local −Y = back)
            parent.spawn((
                Mesh2d(stinger_mesh),
                MeshMaterial2d(stinger_material),
                Transform::from_xyz(0.0, -(PLAYER_RADIUS * 1.5 + 2.0), 0.1),
            ));

            // Left wing
            parent.spawn((
                Mesh2d(wing_mesh.clone()),
                MeshMaterial2d(wing_material.clone()),
                Transform::from_xyz(-(PLAYER_RADIUS + 4.0), 4.0, 0.2),
            ));
            // Right wing
            parent.spawn((
                Mesh2d(wing_mesh),
                MeshMaterial2d(wing_material),
                Transform::from_xyz(PLAYER_RADIUS + 4.0, 4.0, 0.2),
            ));
        });

    // Full-screen black overlay for fade-to-black respawn animation.
    commands.spawn((
        FadeOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Render on top of everything.
        GlobalZIndex(1000),
    ));

    // HUD: controls hint + speed readout in the top-left corner
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top:  Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },
        GlobalZIndex(500),
    ))
    .with_children(|parent| {
        let hint_color = Color::srgba(1.0, 1.0, 1.0, 0.85);
        for line in [
            "🐝  SPACE  – thrust",
            "◀ ▶  Steer",
            "Find the RED flower!",
            "Avoid all other flowers",
            "Fly fast over LAVA",
        ] {
            parent.spawn((
                Text::new(line),
                TextFont { font_size: 16.0, ..default() },
                TextColor(hint_color),
            ));
        }

        // Speed indicator (updated each frame by update_speed_hud)
        parent.spawn((
            SpeedHud,
            Text::new("Speed: 0"),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(1.0, 1.0, 0.4)),
        ));
    });
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Drive the fade-out → teleport → fade-in respawn cycle.
fn handle_respawn(
    time: Res<Time>,
    mut respawn: ResMut<RespawnState>,
    mut player_query: Query<(&mut Transform, &mut BeeVelocity), With<Player>>,
    mut overlay_query: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    let dt = time.delta_secs();
    let Ok(mut overlay) = overlay_query.get_single_mut() else {
        return;
    };

    match respawn.mode {
        RespawnMode::Normal => {}

        RespawnMode::FadingOut => {
            respawn.alpha = (respawn.alpha + dt * FADE_SPEED).min(1.0);
            overlay.0 = Color::srgba(0.0, 0.0, 0.0, respawn.alpha);

            if respawn.alpha >= 1.0 {
                // Teleport back to spawn and zero out velocity.
                if let Ok((mut transform, mut vel)) = player_query.get_single_mut() {
                    transform.translation.x = PLAYER_SPAWN.x;
                    transform.translation.y = PLAYER_SPAWN.y;
                    transform.rotation = Quat::IDENTITY;
                    vel.0 = Vec2::ZERO;
                }
                respawn.mode = RespawnMode::FadingIn;
            }
        }

        RespawnMode::FadingIn => {
            respawn.alpha = (respawn.alpha - dt * FADE_SPEED).max(0.0);
            overlay.0 = Color::srgba(0.0, 0.0, 0.0, respawn.alpha);

            if respawn.alpha <= 0.0 {
                overlay.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                respawn.mode = RespawnMode::Normal;
            }
        }
    }
}

/// Steer and thrust the bee.
///
/// * **Left/Right arrow** – rotate the bee.
/// * **Space** – thrust in the facing direction; velocity decays with drag.
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    respawn: Res<RespawnState>,
    mut query: Query<(&mut Transform, &mut BeeVelocity), With<Player>>,
) {
    if respawn.mode != RespawnMode::Normal {
        return;
    }

    let Ok((mut transform, mut vel)) = query.get_single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // Steering
    if keyboard.pressed(KeyCode::ArrowLeft) {
        transform.rotation *= Quat::from_rotation_z(TURN_SPEED * dt);
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        transform.rotation *= Quat::from_rotation_z(-TURN_SPEED * dt);
    }

    // Thrust: accelerate along the bee's local +Y axis.
    if keyboard.pressed(KeyCode::Space) {
        let forward = (transform.rotation * Vec3::Y).truncate();
        vel.0 += forward * THRUST_ACCEL * dt;
        let speed = vel.0.length();
        if speed > MAX_SPEED {
            vel.0 = vel.0 / speed * MAX_SPEED;
        }
    }

    // Linear drag – bee gradually slows when not thrusting.
    vel.0 *= (1.0 - LINEAR_DRAG * dt).max(0.0);

    // Integrate position.
    transform.translation.x += vel.0.x * dt;
    transform.translation.y += vel.0.y * dt;
}

/// Trigger a respawn if the bee flies over lava while moving too slowly.
fn check_lava(
    lava_tiles:        Option<Res<LavaTiles>>,
    mut respawn_state: ResMut<RespawnState>,
    player_query:      Query<(&Transform, &BeeVelocity), With<Player>>,
) {
    if respawn_state.mode != RespawnMode::Normal {
        return;
    }
    let Some(lava) = lava_tiles else { return; };
    let Ok((transform, vel)) = player_query.get_single() else { return; };

    let pos = transform.translation.truncate();
    if lava.is_over_lava(pos) && vel.0.length() < LAVA_MIN_SPEED {
        respawn_state.mode  = RespawnMode::FadingOut;
        respawn_state.alpha = 0.0;
    }
}

/// Update the speed readout in the HUD every frame.
fn update_speed_hud(
    player_query: Query<&BeeVelocity, With<Player>>,
    mut hud_query: Query<&mut Text, With<SpeedHud>>,
) {
    let Ok(vel) = player_query.get_single() else { return; };
    let Ok(mut text) = hud_query.get_single_mut() else { return; };
    let speed = vel.0.length() as u32;
    *text = Text::new(format!("Speed: {speed}  (lava min: {LAVA_MIN_SPEED})"));
}
