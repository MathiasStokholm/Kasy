use bevy::prelude::*;

use crate::enemy::{Enemy, ENEMY_RADIUS};

pub struct ProjectilePlugin;

/// Speed of a water-gun projectile in world-space units per second.
const PROJECTILE_SPEED: f32 = 400.0;
/// Seconds before an un-hit projectile despawns.
const PROJECTILE_LIFETIME: f32 = 2.5;
/// Visual radius of the projectile in pixels.
const PROJECTILE_RADIUS: f32 = 5.0;
/// Combined hit radius: projectile centre must be within this distance of an
/// enemy centre for a hit to register.
const HIT_RADIUS: f32 = PROJECTILE_RADIUS + ENEMY_RADIUS;

// ---------------------------------------------------------------------------
// Event / component types
// ---------------------------------------------------------------------------

/// Sent by the player system when the left mouse button is clicked.
#[derive(Event)]
pub struct SpawnProjectile {
    /// World-space origin (player position).
    pub position: Vec2,
    /// Normalised direction toward the cursor.
    pub direction: Vec2,
}

/// Runtime state stored on each active projectile entity.
#[derive(Component)]
struct Projectile {
    velocity: Vec2,
    lifetime: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectile>().add_systems(
            Update,
            (
                handle_spawn_projectile,
                move_projectiles,
                check_projectile_hits,
                cleanup_projectiles,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawn a new projectile entity for every received [`SpawnProjectile`] event.
fn handle_spawn_projectile(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut events: EventReader<SpawnProjectile>,
) {
    for event in events.read() {
        commands.spawn((
            Projectile {
                velocity: event.direction * PROJECTILE_SPEED,
                lifetime: PROJECTILE_LIFETIME,
            },
            Mesh2d(meshes.add(Circle::new(PROJECTILE_RADIUS))),
            // Translucent bright blue – looks like a water droplet
            MeshMaterial2d(materials.add(Color::srgba(0.30, 0.60, 1.00, 0.90))),
            // z=15 keeps projectiles in front of tiles (z≈0) and the player (z=10)
            Transform::from_xyz(event.position.x, event.position.y, 15.0),
        ));
    }
}

/// Advance every projectile along its velocity vector each frame.
fn move_projectiles(
    mut query: Query<(&mut Transform, &mut Projectile)>,
    time: Res<Time>,
) {
    for (mut transform, mut projectile) in &mut query {
        let delta = projectile.velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        projectile.lifetime -= time.delta_secs();
    }
}

/// Despawn projectiles whose lifetime has expired.
fn cleanup_projectiles(
    mut commands: Commands,
    query: Query<(Entity, &Projectile)>,
) {
    for (entity, projectile) in &query {
        if projectile.lifetime <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Check every live projectile against every enemy.  On a hit, despawn both.
fn check_projectile_hits(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (proj_entity, proj_transform) in &projectile_query {
        let proj_pos = proj_transform.translation.truncate();
        for (enemy_entity, enemy_transform) in &enemy_query {
            let enemy_pos = enemy_transform.translation.truncate();
            if proj_pos.distance_squared(enemy_pos) < HIT_RADIUS * HIT_RADIUS {
                commands.entity(proj_entity).despawn_recursive();
                commands.entity(enemy_entity).despawn_recursive();
                // Each projectile can only hit one enemy.
                break;
            }
        }
    }
}
