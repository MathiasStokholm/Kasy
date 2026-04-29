use bevy::prelude::*;

use crate::enemy::{Enemy, ENEMY_RADIUS};
use crate::player::{Score, SpawnProjectile, PLAYER_ALTITUDE};

pub struct ProjectilePlugin;

/// Speed of a water-gun projectile in world-space units per second.
const PROJECTILE_SPEED: f32 = 450.0;
/// Seconds before an un-hit projectile despawns.
const PROJECTILE_LIFETIME: f32 = 3.0;
/// Visual radius of the projectile sphere in world-space units.
const PROJECTILE_RADIUS: f32 = 4.0;
/// Combined hit radius for enemy–projectile collision.
const HIT_RADIUS: f32 = PROJECTILE_RADIUS + ENEMY_RADIUS;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Runtime state stored on each active projectile entity.
#[derive(Component)]
struct Projectile {
    /// Velocity in world-space (XZ plane; Y is always 0).
    velocity: Vec3,
    lifetime: f32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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

/// Spawn a new 3-D projectile for every received [`SpawnProjectile`] event.
fn handle_spawn_projectile(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events:   EventReader<SpawnProjectile>,
) {
    for event in events.read() {
        let velocity = Vec3::new(
            event.direction.x * PROJECTILE_SPEED,
            0.0,
            event.direction.y * PROJECTILE_SPEED,
        );
        commands.spawn((
            Projectile { velocity, lifetime: PROJECTILE_LIFETIME },
            Mesh3d(meshes.add(
                Sphere::new(PROJECTILE_RADIUS)
                    .mesh()
                    .ico(2)
                    .expect("projectile ico sphere should build"),
            )),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.30, 0.70, 1.00, 0.90),
                emissive: LinearRgba::rgb(0.5, 1.5, 3.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            // Spawn at the player's XZ position, at flying altitude.
            Transform::from_xyz(event.position.x, PLAYER_ALTITUDE, event.position.y),
        ));
    }
}

/// Advance every projectile along its velocity vector each frame.
fn move_projectiles(
    mut query: Query<(&mut Transform, &mut Projectile)>,
    time: Res<Time>,
) {
    for (mut transform, mut projectile) in &mut query {
        transform.translation += projectile.velocity * time.delta_secs();
        projectile.lifetime   -= time.delta_secs();
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

/// Check every live projectile against every enemy.  On a hit, despawn both
/// and increment the player's score.
fn check_projectile_hits(
    mut commands: Commands,
    mut score: ResMut<Score>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
    enemy_query:      Query<(Entity, &Transform), With<Enemy>>,
) {
    for (proj_entity, proj_tf) in &projectile_query {
        let proj_xz = Vec2::new(proj_tf.translation.x, proj_tf.translation.z);
        for (enemy_entity, enemy_tf) in &enemy_query {
            let enemy_xz = Vec2::new(enemy_tf.translation.x, enemy_tf.translation.z);
            if proj_xz.distance_squared(enemy_xz) < HIT_RADIUS * HIT_RADIUS {
                commands.entity(proj_entity).despawn_recursive();
                commands.entity(enemy_entity).despawn_recursive();
                score.enemies_defeated += 1;
                // Each projectile can only hit one enemy.
                break;
            }
        }
    }
}
