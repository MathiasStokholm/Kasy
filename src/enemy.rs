use bevy::prelude::*;

use crate::iso::grid_to_world;
use crate::player::{Player, PLAYER_RADIUS, RespawnMode, RespawnState};
use crate::world::WorldTiles;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawner>()
            .add_systems(Startup, spawn_initial_enemies)
            .add_systems(Update, (enemy_wander, check_enemy_player_collision, tick_spawner));
    }
}

// ---------------------------------------------------------------------------
// Component / constants
// ---------------------------------------------------------------------------

/// Collision radius of an enemy sphere in world-space units.
pub const ENEMY_RADIUS: f32 = 10.0;
/// Base wandering speed in world-space units per second (at t = 0).
const ENEMY_SPEED: f32 = 55.0;
/// Base chase speed – faster than wandering to make the threat credible.
const ENEMY_CHASE_SPEED: f32 = 90.0;
/// Distance at which an enemy switches from wandering to chasing the player.
const CHASE_RANGE: f32 = 130.0;
/// Minimum distance to the player before an enemy stops moving toward them,
/// preventing jitter when the enemy is already on top of the player.
const MIN_CHASE_DISTANCE: f32 = 0.5;
/// How long (seconds) an enemy travels in one direction before picking a new one.
const WANDER_INTERVAL: f32 = 2.5;
/// Height of enemies above the ground plane.
const ENEMY_ALTITUDE: f32 = 8.0;

// -- Spawner tuning ---------------------------------------------------------
/// Starting spawn interval (seconds between new enemies).
const SPAWN_INTERVAL_START: f32 = 6.0;
/// Minimum spawn interval – the fastest the game will keep creating enemies.
const SPAWN_INTERVAL_MIN: f32 = 1.2;
/// Time (seconds) at which the spawn interval has halved from its start value.
const SPAWN_INTERVAL_HALF_TIME: f32 = 60.0;
/// Elapsed game-seconds after which enemy base speed doubles.
const SPEED_DOUBLE_TIME: f32 = 120.0;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks the periodic enemy-spawner state.
#[derive(Resource)]
pub struct EnemySpawner {
    /// Countdown until the next enemy is spawned (seconds).
    timer:        f32,
    /// Total elapsed play-time (seconds); drives rate & speed scaling.
    elapsed_secs: f32,
    /// LCG seed for picking random spawn points.
    rng_seed:     u32,
}

impl Default for EnemySpawner {
    fn default() -> Self {
        Self {
            timer:        SPAWN_INTERVAL_START,
            elapsed_secs: 0.0,
            rng_seed:     0xDEAD_BEEF,
        }
    }
}

impl EnemySpawner {
    /// Current spawn interval based on elapsed time.
    fn interval(&self) -> f32 {
        let t = self.elapsed_secs;
        (SPAWN_INTERVAL_START / (1.0 + t / SPAWN_INTERVAL_HALF_TIME))
            .max(SPAWN_INTERVAL_MIN)
    }

    /// Wander speed for a newly spawned enemy.
    fn enemy_speed(&self) -> f32 {
        ENEMY_SPEED * (1.0 + self.elapsed_secs / SPEED_DOUBLE_TIME)
    }

    /// Chase speed for a newly spawned enemy.
    fn chase_speed(&self) -> f32 {
        ENEMY_CHASE_SPEED * (1.0 + self.elapsed_secs / SPEED_DOUBLE_TIME)
    }

    /// Pick a pseudo-random spawn point.
    fn next_spawn_point(&mut self) -> (i32, i32) {
        self.rng_seed = lcg(self.rng_seed);
        SPAWN_POINTS[self.rng_seed as usize % SPAWN_POINTS.len()]
    }
}

// ---------------------------------------------------------------------------
// Enemy component
// ---------------------------------------------------------------------------

/// Enemy entity with its own lightweight PRNG seed for deterministic wandering.
#[derive(Component)]
pub struct Enemy {
    velocity:     Vec2,
    wander_timer: f32,
    rng_seed:     u32,
    /// Base wander speed for this individual enemy (set at spawn time).
    speed:        f32,
    /// Base chase speed for this individual enemy (set at spawn time).
    chase_speed:  f32,
}

// ---------------------------------------------------------------------------
// Spawn positions (isometric grid coordinates)
// ---------------------------------------------------------------------------

/// One entry per possible spawn location; grid coords chosen to be safely
/// inside each island.
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
// Shared helpers
// ---------------------------------------------------------------------------

/// Build and insert a single enemy entity at grid position `(gx, gy)`.
fn spawn_enemy_at(
    commands: &mut Commands,
    mesh:     &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    gx: i32,
    gy: i32,
    speed:       f32,
    chase_speed: f32,
    rng_seed:    u32,
) {
    let world_pos = grid_to_world(gx, gy);
    let angle     = (rng_seed % 628) as f32 / 100.0;

    commands.spawn((
        Enemy {
            velocity:     Vec2::new(angle.cos(), angle.sin()) * speed,
            wander_timer: (rng_seed % 250) as f32 / 100.0,
            rng_seed,
            speed,
            chase_speed,
        },
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(world_pos.x, ENEMY_ALTITUDE, world_pos.y),
    ));
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawn the initial set of enemies at game start (all spawn points, base speed).
fn spawn_initial_enemies(
    mut commands: Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh     = create_enemy_mesh(&mut meshes);
    let material = create_enemy_material(&mut materials);

    for &(gx, gy) in SPAWN_POINTS {
        let seed = lcg(((gx as u32).wrapping_mul(LCG_MUL))
            .wrapping_add((gy as u32).wrapping_mul(LCG_INC)));
        spawn_enemy_at(
            &mut commands, &mesh, &material,
            gx, gy,
            ENEMY_SPEED, ENEMY_CHASE_SPEED,
            seed,
        );
    }
}

/// Tick the spawner and create a new enemy whenever the interval elapses.
fn tick_spawner(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawner:   ResMut<EnemySpawner>,
    time:          Res<Time>,
) {
    let dt = time.delta_secs();
    spawner.elapsed_secs += dt;
    spawner.timer        -= dt;

    if spawner.timer <= 0.0 {
        let interval = spawner.interval();
        spawner.timer = interval;

        let (gx, gy)  = spawner.next_spawn_point();
        let speed      = spawner.enemy_speed();
        let chase_spd  = spawner.chase_speed();
        let seed       = spawner.rng_seed;

        let mesh     = create_enemy_mesh(&mut meshes);
        let material = create_enemy_material(&mut materials);

        spawn_enemy_at(&mut commands, &mesh, &material, gx, gy, speed, chase_spd, seed);
    }
}

/// Move each enemy.  Enemies chase the player when nearby; otherwise they
/// wander randomly, staying on the islands.
fn enemy_wander(
    time:         Res<Time>,
    world_tiles:  Option<Res<WorldTiles>>,
    player_query: Query<&Transform, With<Player>>,
    mut query:    Query<(&mut Transform, &mut Enemy), Without<Player>>,
) {
    let Some(world_tiles) = world_tiles else { return; };
    let dt = time.delta_secs();

    let player_xz = player_query
        .get_single()
        .map(|t| Vec2::new(t.translation.x, t.translation.z))
        .ok();

    for (mut transform, mut enemy) in &mut query {
        enemy.wander_timer -= dt;

        let enemy_xz = Vec2::new(transform.translation.x, transform.translation.z);

        let (chasing, move_vel) = match player_xz {
            Some(pxz) => {
                let to_player = pxz - enemy_xz;
                let dist = to_player.length();
                if dist < CHASE_RANGE && dist > MIN_CHASE_DISTANCE {
                    (true, to_player.normalize() * enemy.chase_speed)
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

        if !chasing && (!on_tile || enemy.wander_timer <= 0.0) {
            enemy.rng_seed     = lcg(enemy.rng_seed);
            let angle          = (enemy.rng_seed % 628) as f32 / 100.0;
            enemy.velocity     = Vec2::new(angle.cos(), angle.sin()) * enemy.speed;
            enemy.wander_timer = WANDER_INTERVAL;
        } else if chasing && enemy.wander_timer <= 0.0 {
            enemy.rng_seed     = lcg(enemy.rng_seed);
            let angle          = (enemy.rng_seed % 628) as f32 / 100.0;
            enemy.velocity     = Vec2::new(angle.cos(), angle.sin()) * enemy.speed;
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
// Mesh / material helpers
// ---------------------------------------------------------------------------

fn create_enemy_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    meshes.add(
        Sphere::new(ENEMY_RADIUS)
            .mesh()
            .ico(3)
            .expect("enemy ico sphere should build"),
    )
}

fn create_enemy_material(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::srgb(0.90, 0.25, 0.05),
        emissive: LinearRgba::rgb(1.8, 0.4, 0.0),
        perceptual_roughness: 0.55,
        ..default()
    })
}

// ---------------------------------------------------------------------------
// LCG helpers
// ---------------------------------------------------------------------------

const LCG_MUL: u32 = 1_664_525;
const LCG_INC: u32 = 1_013_904_223;

#[inline]
fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(LCG_MUL).wrapping_add(LCG_INC)
}
