use bevy::prelude::*;

pub struct ProjectilePlugin;

/// Speed of a water-gun projectile in world-space units per second.
const PROJECTILE_SPEED: f32 = 400.0;
/// Seconds before an un-hit projectile despawns.
const PROJECTILE_LIFETIME: f32 = 2.5;
/// Visual radius of the projectile in pixels.
const PROJECTILE_RADIUS: f32 = 5.0;

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
