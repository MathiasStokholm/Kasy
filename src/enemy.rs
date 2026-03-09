use bevy::prelude::*;

use crate::iso::{grid_to_depth, grid_to_world};
use crate::world::WorldTiles;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemies)
            .add_systems(Update, enemy_wander);
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Visual radius of an enemy circle sprite in pixels.
pub const ENEMY_RADIUS: f32 = 12.0;
/// Movement speed in world-space units per second.
const ENEMY_SPEED: f32 = 55.0;
/// How long (seconds) an enemy travels in one direction before picking a new one.
const WANDER_INTERVAL: f32 = 2.5;

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
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // All enemies share the same mesh/material handles.
    let mesh     = meshes.add(Circle::new(ENEMY_RADIUS));
    let material = materials.add(Color::srgb(0.90, 0.45, 0.05));

    for &(gx, gy) in SPAWN_POINTS {
        let world_pos = grid_to_world(gx, gy);
        // Place just below the player (z=10) but above tiles.
        let z = grid_to_depth(gx, gy) + 9.0;

        // Seed each enemy differently so they wander in different directions
        // from the first frame.
        let seed = lcg(((gx as u32).wrapping_mul(LCG_MUL))
            .wrapping_add((gy as u32).wrapping_mul(LCG_INC)));
        let angle = (seed % 628) as f32 / 100.0; // 0 .. ~2π

        commands.spawn((
            Enemy {
                velocity:     Vec2::new(angle.cos(), angle.sin()) * ENEMY_SPEED,
                wander_timer: (seed % 250) as f32 / 100.0, // stagger initial turns
                rng_seed:     seed,
            },
            Mesh2d(mesh.clone()),
            MeshMaterial2d(material.clone()),
            Transform::from_xyz(world_pos.x, world_pos.y + 8.0, z),
        ));
    }
}

/// Move each enemy in its current direction.  When the timer expires, or when
/// the enemy would step off the islands, a new direction is chosen.
fn enemy_wander(
    time:        Res<Time>,
    world_tiles: Option<Res<WorldTiles>>,
    mut query:   Query<(&mut Transform, &mut Enemy)>,
) {
    let Some(world_tiles) = world_tiles else { return; };
    let dt = time.delta_secs();

    for (mut transform, mut enemy) in &mut query {
        // Count down the wander timer.
        enemy.wander_timer -= dt;

        // Try to move forward.
        let new_x = transform.translation.x + enemy.velocity.x * dt;
        let new_y = transform.translation.y + enemy.velocity.y * dt;

        let on_tile = world_tiles.is_over_tile(Vec2::new(new_x, new_y));

        if on_tile && enemy.wander_timer > 0.0 {
            // Normal move.
            transform.translation.x = new_x;
            transform.translation.y = new_y;
        } else {
            // Either the timer expired or we'd step off the island – pick a
            // new random direction and reset the timer.
            enemy.rng_seed       = lcg(enemy.rng_seed);
            let angle            = (enemy.rng_seed % 628) as f32 / 100.0;
            enemy.velocity       = Vec2::new(angle.cos(), angle.sin()) * ENEMY_SPEED;
            enemy.wander_timer   = WANDER_INTERVAL;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Knuth's LCG multiplier and increment (classic 32-bit Park–Miller constants).
const LCG_MUL: u32 = 1_664_525;
const LCG_INC: u32 = 1_013_904_223;

/// Minimal linear-congruential PRNG (no external dependencies needed).
#[inline]
fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(LCG_MUL).wrapping_add(LCG_INC)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_zero_seed_returns_increment() {
        // lcg(0) = 0 * LCG_MUL + LCG_INC = LCG_INC
        assert_eq!(lcg(0), LCG_INC);
    }

    #[test]
    fn lcg_unit_seed_returns_mul_plus_inc() {
        assert_eq!(lcg(1), LCG_MUL.wrapping_add(LCG_INC));
    }

    #[test]
    fn lcg_is_deterministic() {
        assert_eq!(lcg(42), lcg(42));
    }

    #[test]
    fn lcg_produces_varied_output() {
        // Consecutive outputs must differ (otherwise the PRNG is degenerate).
        let v1 = lcg(1234);
        let v2 = lcg(v1);
        assert_ne!(v1, v2);
    }

    #[test]
    fn lcg_wraps_without_panic() {
        // Should not panic on overflow due to wrapping arithmetic.
        let _ = lcg(u32::MAX);
    }
}
