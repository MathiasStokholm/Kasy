use bevy::prelude::*;

use crate::iso::grid_to_world;
use crate::player::{Player, PLAYER_RADIUS, RespawnMode, RespawnState};
use crate::world::WorldTiles;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemies)
            .add_systems(Update, (enemy_wander, check_enemy_player_collision));
    }
}

// ---------------------------------------------------------------------------
// Component / constants
// ---------------------------------------------------------------------------

/// Collision radius of an enemy sphere in world-space units.
pub const ENEMY_RADIUS: f32 = 10.0;
/// Normal wandering speed in world-space units per second.
const ENEMY_SPEED: f32 = 55.0;
/// Chase speed – faster than wandering to make the threat credible.
const ENEMY_CHASE_SPEED: f32 = 90.0;
/// Distance at which an enemy switches from wandering to chasing the player.
const CHASE_RANGE: f32 = 130.0;
/// How long (seconds) an enemy travels in one direction before picking a new one.
const WANDER_INTERVAL: f32 = 2.5;
/// Height of enemies above the ground plane.
const ENEMY_ALTITUDE: f32 = 8.0;

/// Enemy entity with its own lightweight PRNG seed for deterministic wandering.
#[derive(Component)]
pub struct Enemy {
    velocity:     Vec2,
    wander_timer: f32,
    rng_seed:     u32,
}

// ---------------------------------------------------------------------------
// Spawn positions (isometric grid coordinates)
// ---------------------------------------------------------------------------

/// One enemy per entry; grid coords chosen to be safely inside each island.
const SPAWN_POINTS: &[(i32, i32)] = &[
    // Grasslands hub
    (6, 3), (-5, 7), (9, -4),
    // Jungle island
    (43, 37), (40, 41), (46, 35),
    // Desert island
    (57, -38), (52, -43),
    // Tundra / snow island
    (-46, -47), (-51, -41),
    // Volcanic island
    (-45, 49), (-42, 44),
    // Rocky outcrop
    (5, -57), (4, -51),
];

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn spawn_enemies(
    mut commands: Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(
        Sphere::new(ENEMY_RADIUS)
            .mesh()
            .ico(3)
            .expect("enemy ico sphere should build"),
    );
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.25, 0.05),
        emissive: LinearRgba::rgb(1.8, 0.4, 0.0),
        perceptual_roughness: 0.55,
        ..default()
    });

    for &(gx, gy) in SPAWN_POINTS {
        let world_pos = grid_to_world(gx, gy);

        let seed = lcg(((gx as u32).wrapping_mul(LCG_MUL))
            .wrapping_add((gy as u32).wrapping_mul(LCG_INC)));
        let angle = (seed % 628) as f32 / 100.0;

        commands.spawn((
            Enemy {
                velocity:     Vec2::new(angle.cos(), angle.sin()) * ENEMY_SPEED,
                wander_timer: (seed % 250) as f32 / 100.0,
                rng_seed:     seed,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            // world_pos is (screen_x, iso_y); in 3D that maps to (x, altitude, z).
            Transform::from_xyz(world_pos.x, ENEMY_ALTITUDE, world_pos.y),
        ));
    }
}

/// Move each enemy.  Enemies chase the player when nearby; otherwise they
/// wander randomly, staying on the islands.
fn enemy_wander(
    time:        Res<Time>,
    world_tiles: Option<Res<WorldTiles>>,
    player_query: Query<&Transform, With<Player>>,
    mut query:   Query<(&mut Transform, &mut Enemy), Without<Player>>,
) {
    let Some(world_tiles) = world_tiles else { return; };
    let dt = time.delta_secs();

    // Snapshot the player's XZ position once (avoids re-querying per enemy).
    let player_xz = player_query
        .get_single()
        .map(|t| Vec2::new(t.translation.x, t.translation.z))
        .ok();

    for (mut transform, mut enemy) in &mut query {
        enemy.wander_timer -= dt;

        let enemy_xz = Vec2::new(transform.translation.x, transform.translation.z);

        // Decide whether to chase or wander.
        let (chasing, move_vel) = match player_xz {
            Some(pxz) => {
                let to_player = pxz - enemy_xz;
                let dist = to_player.length();
                if dist < CHASE_RANGE && dist > 0.5 {
                    (true, to_player.normalize() * ENEMY_CHASE_SPEED)
                } else {
                    (false, enemy.velocity)
                }
            }
            None => (false, enemy.velocity),
        };

        let new_x = transform.translation.x + move_vel.x * dt;
        let new_z = transform.translation.z + move_vel.y * dt;
        let on_tile = world_tiles.is_over_tile(Vec2::new(new_x, new_z));

        if on_tile {
            transform.translation.x = new_x;
            transform.translation.z = new_z;
        }

        // Update wander direction when the timer expires or when wandering
        // would take the enemy off an island edge.
        if !chasing && (!on_tile || enemy.wander_timer <= 0.0) {
            enemy.rng_seed     = lcg(enemy.rng_seed);
            let angle          = (enemy.rng_seed % 628) as f32 / 100.0;
            enemy.velocity     = Vec2::new(angle.cos(), angle.sin()) * ENEMY_SPEED;
            enemy.wander_timer = WANDER_INTERVAL;
        } else if chasing && enemy.wander_timer <= 0.0 {
            // Keep wander direction fresh in the background even while chasing.
            enemy.rng_seed     = lcg(enemy.rng_seed);
            let angle          = (enemy.rng_seed % 628) as f32 / 100.0;
            enemy.velocity     = Vec2::new(angle.cos(), angle.sin()) * ENEMY_SPEED;
            enemy.wander_timer = WANDER_INTERVAL;
        }
    }
}

/// Trigger a respawn if an enemy touches the player.
fn check_enemy_player_collision(
    mut respawn_state: ResMut<RespawnState>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query:  Query<&Transform, With<Enemy>>,
) {
    if respawn_state.won || respawn_state.mode != RespawnMode::Normal {
        return;
    }
    let Ok(player_tf) = player_query.get_single() else { return; };
    let player_xz = Vec2::new(player_tf.translation.x, player_tf.translation.z);
    let collision_dist = PLAYER_RADIUS + ENEMY_RADIUS + 2.0;

    for enemy_tf in &enemy_query {
        let enemy_xz = Vec2::new(enemy_tf.translation.x, enemy_tf.translation.z);
        if player_xz.distance_squared(enemy_xz) < collision_dist * collision_dist {
            respawn_state.mode  = RespawnMode::FadingOut;
            respawn_state.alpha = 0.0;
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const LCG_MUL: u32 = 1_664_525;
const LCG_INC: u32 = 1_013_904_223;

#[inline]
fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(LCG_MUL).wrapping_add(LCG_INC)
}
