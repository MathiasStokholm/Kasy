use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{iso::world_to_plane, world::LavaTiles};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RespawnState>()
            .init_resource::<Score>()
            .add_event::<SpawnProjectile>()
            .add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (
                    handle_respawn,
                    player_movement.after(handle_respawn),
                    player_aim_and_shoot.after(handle_respawn),
                    check_lava.after(player_movement),
                    update_speed_hud.after(player_movement),
                    update_score_hud,
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

/// Marker for the score text in the HUD.
#[derive(Component)]
struct ScoreHud;

/// Event fired when the player wants to fire the water gun.
#[derive(Event)]
pub struct SpawnProjectile {
    /// World-space origin (player XZ position).
    pub position: Vec2,
    /// Normalised direction in the XZ plane toward the cursor.
    pub direction: Vec2,
}

/// Tracks the number of enemies defeated this session.
#[derive(Resource, Default)]
pub struct Score {
    pub enemies_defeated: u32,
}

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
    pub mode: RespawnMode,
    /// Current overlay alpha (0 = transparent, 1 = fully black).
    pub alpha: f32,
    /// Set to `true` while the win overlay is displayed; prevents other
    /// hazards (enemies, flowers, lava) from interfering with the win screen.
    pub won: bool,
}

/// Radius of the bee body sprite in pixels (used for collision checks).
pub const PLAYER_RADIUS: f32 = 10.0;

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
pub const PLAYER_ALTITUDE: f32 = 12.0;

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(Cuboid::new(12.0, 7.0, 18.0));
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.82, 0.08),
        perceptual_roughness: 0.55,
        ..default()
    });

    let stripe_mesh = meshes.add(Cuboid::new(12.5, 1.2, 3.0));
    let stripe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.85,
        ..default()
    });

    let stinger_mesh = meshes.add(Cuboid::new(3.0, 2.0, 4.0));
    let stinger_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.10, 0.05),
        perceptual_roughness: 0.75,
        ..default()
    });

    let wing_mesh = meshes.add(Sphere::new(4.5).mesh().ico(4).unwrap());
    let wing_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.80, 0.92, 1.0, 0.35),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.15,
        reflectance: 0.65,
        ..default()
    });

    commands
        .spawn((
            Player,
            BeeVelocity(Vec2::ZERO),
            Mesh3d(body_mesh),
            MeshMaterial3d(body_material),
            Transform::from_xyz(PLAYER_SPAWN.x, PLAYER_ALTITUDE, PLAYER_SPAWN.y),
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
            ViewVisibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(stripe_mesh.clone()),
                MeshMaterial3d(stripe_material.clone()),
                Transform::from_xyz(0.0, 1.8, -3.0),
            ));
            parent.spawn((
                Mesh3d(stripe_mesh),
                MeshMaterial3d(stripe_material),
                Transform::from_xyz(0.0, 1.8, 3.0),
            ));

            parent.spawn((
                Mesh3d(stinger_mesh),
                MeshMaterial3d(stinger_material),
                Transform::from_xyz(0.0, -0.5, 10.0),
            ));

            parent.spawn((
                Mesh3d(wing_mesh.clone()),
                MeshMaterial3d(wing_material.clone()),
                Transform::from_xyz(-6.5, 4.0, -1.0).with_scale(Vec3::new(1.2, 0.45, 0.9)),
            ));
            parent.spawn((
                Mesh3d(wing_mesh),
                MeshMaterial3d(wing_material),
                Transform::from_xyz(6.5, 4.0, -1.0).with_scale(Vec3::new(1.2, 0.45, 0.9)),
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
        GlobalZIndex(1000),
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(12.0),
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
                "🖱  Mouse  – aim & steer",
                "🖱  Click  – fire water gun!",
                "Find the RED flower!",
                "Avoid all other flowers",
                "Fly fast over LAVA",
                "Shoot enemies for points!",
            ] {
                parent.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(hint_color),
                ));
            }

            parent.spawn((
                SpeedHud,
                Text::new("Speed: 0"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.4)),
            ));

            parent.spawn((
                ScoreHud,
                Text::new("Enemies defeated: 0"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 1.0, 0.5)),
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
                if let Ok((mut transform, mut vel)) = player_query.get_single_mut() {
                    transform.translation = Vec3::new(PLAYER_SPAWN.x, PLAYER_ALTITUDE, PLAYER_SPAWN.y);
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

/// Thrust the bee forward; the bee's facing direction is controlled by the mouse.
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

    if keyboard.pressed(KeyCode::Space) {
        let forward = transform.rotation * -Vec3::Z;
        vel.0 += Vec2::new(forward.x, forward.z) * THRUST_ACCEL * dt;
        let speed = vel.0.length();
        if speed > MAX_SPEED {
            vel.0 = vel.0 / speed * MAX_SPEED;
        }
    }

    vel.0 *= (1.0 - LINEAR_DRAG * dt).max(0.0);
    transform.translation.x += vel.0.x * dt;
    transform.translation.z += vel.0.y * dt;
}

/// Rotate the bee toward the mouse cursor and fire a projectile on left-click.
fn player_aim_and_shoot(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<(&mut Transform, &BeeVelocity), With<Player>>,
    buttons: Res<ButtonInput<MouseButton>>,
    respawn: Res<RespawnState>,
    mut spawn_events: EventWriter<SpawnProjectile>,
) {
    if respawn.mode != RespawnMode::Normal {
        return;
    }

    let Ok(window) = windows.get_single() else { return; };
    let Ok((camera, cam_transform)) = camera_query.get_single() else { return; };
    let Ok((mut player_tf, _vel)) = player_query.get_single_mut() else { return; };
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Ok(ray) = camera.viewport_to_world(cam_transform, cursor_pos) else { return; };

    // Intersect the camera ray with the horizontal plane at player altitude.
    let dir = ray.direction.as_vec3();
    if dir.y.abs() < 0.0001 {
        return;
    }
    let t = (PLAYER_ALTITUDE - ray.origin.y) / dir.y;
    if t <= 0.0 {
        return;
    }

    let aim_world = ray.origin + dir * t;
    let player_xz = Vec2::new(player_tf.translation.x, player_tf.translation.z);
    let aim_xz = Vec2::new(aim_world.x, aim_world.z);
    let aim_dir = (aim_xz - player_xz).normalize_or_zero();

    if aim_dir.length_squared() > 0.01 {
        // Rotate bee to face aim direction (-Z is the bee's forward).
        // Desired yaw: -Z maps to (aim_dir.x, aim_dir.y) in XZ.
        let yaw = f32::atan2(-aim_dir.x, -aim_dir.y);
        player_tf.rotation = Quat::from_rotation_y(yaw);
    }

    if buttons.just_pressed(MouseButton::Left) && aim_dir.length_squared() > 0.01 {
        spawn_events.send(SpawnProjectile {
            position: player_xz,
            direction: aim_dir,
        });
    }
}

/// Trigger a respawn if the bee flies over lava while moving too slowly.
fn check_lava(
    lava_tiles: Option<Res<LavaTiles>>,
    mut respawn_state: ResMut<RespawnState>,
    player_query: Query<(&Transform, &BeeVelocity), With<Player>>,
) {
    if respawn_state.won || respawn_state.mode != RespawnMode::Normal {
        return;
    }
    let Some(lava) = lava_tiles else {
        return;
    };
    let Ok((transform, vel)) = player_query.get_single() else {
        return;
    };

    if lava.is_over_lava(world_to_plane(transform.translation)) && vel.0.length() < LAVA_MIN_SPEED {
        respawn_state.mode = RespawnMode::FadingOut;
        respawn_state.alpha = 0.0;
    }
}

/// Update the speed readout in the HUD every frame.
fn update_speed_hud(
    player_query: Query<&BeeVelocity, With<Player>>,
    mut hud_query: Query<&mut Text, With<SpeedHud>>,
) {
    let Ok(vel) = player_query.get_single() else {
        return;
    };
    let Ok(mut text) = hud_query.get_single_mut() else {
        return;
    };
    let speed = vel.0.length() as u32;
    *text = Text::new(format!("Speed: {speed}  (lava min: {LAVA_MIN_SPEED})"));
}

/// Update the enemies-defeated counter in the HUD every frame.
fn update_score_hud(
    score: Res<Score>,
    mut hud_query: Query<&mut Text, With<ScoreHud>>,
) {
    let Ok(mut text) = hud_query.get_single_mut() else {
        return;
    };
    *text = Text::new(format!("Enemies defeated: {}", score.enemies_defeated));
}
