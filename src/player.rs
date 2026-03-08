use bevy::{prelude::*, window::PrimaryWindow};

use crate::projectile::SpawnProjectile;
use crate::world::WorldTiles;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FallState>()
            .add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (
                    handle_player_fall,
                    player_movement.after(handle_player_fall),
                    player_aim.after(handle_player_fall),
                    player_shoot.after(handle_player_fall),
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

/// Marker for the full-screen black overlay used for the fade-to-black effect.
#[derive(Component)]
struct FadeOverlay;

/// Which phase of the fall-and-respawn sequence we are in.
#[derive(Default, PartialEq)]
enum FallMode {
    /// Normal gameplay – player is on the islands.
    #[default]
    Normal,
    /// Player has walked off an island; fading the screen to black.
    FadingOut,
    /// Screen is fully black; fading back in at the spawn point.
    FadingIn,
}

/// Tracks fall / respawn state.  Stored as a resource so all systems share it.
#[derive(Resource, Default)]
pub struct FallState {
    mode:  FallMode,
    /// Current overlay alpha (0 = transparent, 1 = fully black).
    alpha: f32,
}

/// Movement speed in world-space units per second.
const PLAYER_SPEED: f32 = 200.0;
/// Radius of the player circle sprite in pixels.
const PLAYER_RADIUS: f32 = 14.0;
/// Gap between the player body edge and the start of the gun barrel.
const BARREL_OFFSET: f32 = 11.0;
/// Speed of the fade-to-black transition (alpha units per second).
const FADE_SPEED: f32 = 2.5;
/// World-space spawn position (matches the initial `Transform` in `setup_player`).
const PLAYER_SPAWN: Vec2 = Vec2::new(0.0, 8.0);

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let player_mesh = meshes.add(Circle::new(PLAYER_RADIUS));
    let player_material = materials.add(Color::srgb(0.85, 0.25, 0.25));

    // The gun barrel is a thin rectangle that sticks out in the player's
    // local +Y direction.  Because the player entity rotates to face the
    // mouse, the barrel always points toward the cursor.
    let barrel_mesh = meshes.add(Rectangle::new(5.0, 22.0));
    let barrel_material = materials.add(Color::srgb(0.20, 0.50, 0.95));

    commands
        .spawn((
            Player,
            Mesh2d(player_mesh),
            MeshMaterial2d(player_material),
            // Start above the centre tile; z=10 ensures the player is always
            // rendered on top of all tile geometry.
            Transform::from_xyz(PLAYER_SPAWN.x, PLAYER_SPAWN.y, 10.0),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh2d(barrel_mesh),
                MeshMaterial2d(barrel_material),
                // Offset the barrel forward (local +Y) by the player radius
                // plus a small gap so it appears to protrude from the body.
                Transform::from_xyz(0.0, PLAYER_RADIUS + BARREL_OFFSET, 0.5),
            ));
        });

    // Full-screen black overlay for fade-to-black.  Starts transparent and is
    // brought to alpha=1 when the player falls off an island.
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
        // Render on top of all 2-D game geometry.
        GlobalZIndex(1000),
    ));
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Detect when the player has walked off the islands and drive the
/// fade-out → teleport → fade-in cycle.
fn handle_player_fall(
    time: Res<Time>,
    world_tiles: Option<Res<WorldTiles>>,
    mut fall_state: ResMut<FallState>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut overlay_query: Query<&mut BackgroundColor, With<FadeOverlay>>,
) {
    let dt = time.delta_secs();
    let Ok(mut overlay) = overlay_query.get_single_mut() else {
        return;
    };

    match fall_state.mode {
        FallMode::Normal => {
            // Only start falling when the WorldTiles resource is available
            // (it is inserted in the same Startup frame, so this is always true
            // during Update, but the `Option` guards against edge cases).
            if let Some(tiles) = world_tiles {
                if let Ok(transform) = player_query.get_single() {
                    let pos = transform.translation.truncate();
                    if !tiles.is_over_tile(pos) {
                        fall_state.mode  = FallMode::FadingOut;
                        fall_state.alpha = 0.0;
                    }
                }
            }
        }

        FallMode::FadingOut => {
            fall_state.alpha = (fall_state.alpha + dt * FADE_SPEED).min(1.0);
            overlay.0 = Color::srgba(0.0, 0.0, 0.0, fall_state.alpha);

            if fall_state.alpha >= 1.0 {
                // Screen is fully black – teleport back to spawn.
                if let Ok(mut transform) = player_query.get_single_mut() {
                    transform.translation.x = PLAYER_SPAWN.x;
                    transform.translation.y = PLAYER_SPAWN.y;
                    // Reset rotation so the barrel points up.
                    transform.rotation = Quat::IDENTITY;
                }
                fall_state.mode = FallMode::FadingIn;
            }
        }

        FallMode::FadingIn => {
            fall_state.alpha = (fall_state.alpha - dt * FADE_SPEED).max(0.0);
            overlay.0 = Color::srgba(0.0, 0.0, 0.0, fall_state.alpha);

            if fall_state.alpha <= 0.0 {
                overlay.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                fall_state.mode = FallMode::Normal;
            }
        }
    }
}

/// Move the player in screen-space directions based on WASD keys.
///
/// Movement is independent of the player's facing direction (strafing style):
/// W/S move along the screen Y axis and A/D along the screen X axis.
/// Blocked during fall/respawn.
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    fall_state: Res<FallState>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    if fall_state.mode != FallMode::Normal {
        return;
    }

    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        transform.translation.x += direction.x * PLAYER_SPEED * time.delta_secs();
        transform.translation.y += direction.y * PLAYER_SPEED * time.delta_secs();
    }
}

/// Rotate the player to face the current mouse-cursor position.
/// Skipped during fall/respawn.
fn player_aim(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<&mut Transform, With<Player>>,
    fall_state: Res<FallState>,
) {
    if fall_state.mode != FallMode::Normal {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    let Ok(mut player_transform) = player_query.get_single_mut() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let to_cursor = world_pos - player_transform.translation.truncate();
    if to_cursor.length_squared() > 1.0 {
        // atan2 gives angle from +X axis; subtract π/2 to align our sprite's
        // local +Y axis (the barrel) with the direction to the cursor.
        let angle = to_cursor.y.atan2(to_cursor.x) - std::f32::consts::FRAC_PI_2;
        player_transform.rotation = Quat::from_rotation_z(angle);
    }
}

/// Fire a water-gun projectile toward the cursor on left-click.
/// Blocked during fall/respawn.
fn player_shoot(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    player_query: Query<&Transform, With<Player>>,
    mut spawn_events: EventWriter<SpawnProjectile>,
    fall_state: Res<FallState>,
) {
    if fall_state.mode != FallMode::Normal {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.get_single() else {
        return;
    };
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let player_pos = player_transform.translation.truncate();
    let direction = (world_pos - player_pos).normalize_or_zero();

    if direction != Vec2::ZERO {
        spawn_events.send(SpawnProjectile {
            position: player_pos,
            direction,
        });
    }
}
