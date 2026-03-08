use bevy::{prelude::*, window::PrimaryWindow};

use crate::projectile::SpawnProjectile;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
            .add_systems(Update, (player_movement, player_aim, player_shoot));
    }
}

// ---------------------------------------------------------------------------
// Components / constants
// ---------------------------------------------------------------------------

/// Marker component that identifies the player entity.
#[derive(Component)]
pub struct Player;

/// Movement speed in world-space units per second.
const PLAYER_SPEED: f32 = 200.0;
/// Radius of the player circle sprite in pixels.
const PLAYER_RADIUS: f32 = 14.0;
/// Gap between the player body edge and the start of the gun barrel.
const BARREL_OFFSET: f32 = 11.0;

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
            Transform::from_xyz(0.0, 8.0, 10.0),
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
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Move the player in screen-space directions based on WASD keys.
///
/// Movement is independent of the player's facing direction (strafing style):
/// W/S move along the screen Y axis and A/D along the screen X axis.
fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Player>>,
) {
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
fn player_aim(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
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
fn player_shoot(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    player_query: Query<&Transform, With<Player>>,
    mut spawn_events: EventWriter<SpawnProjectile>,
) {
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
