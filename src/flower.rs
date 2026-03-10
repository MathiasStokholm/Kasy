use bevy::prelude::*;

use crate::iso::{grid_to_depth, grid_to_world};
use crate::player::{BeeVelocity, Player, PLAYER_RADIUS, PLAYER_SPAWN, RespawnMode, RespawnState};

pub struct FlowerPlugin;

impl Plugin for FlowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_flowers)
            .add_systems(Update, (check_flower_collision, check_win_collision));
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Visual radius of a non-red flower sprite.
const FLOWER_RADIUS: f32 = 6.0;
/// Visual radius of the special red flower (slightly larger to stand out).
const RED_FLOWER_RADIUS: f32 = 9.0;
/// Distance (bee centre to flower centre) at which a collision is registered.
const COLLISION_DIST_NORMAL: f32 = PLAYER_RADIUS + FLOWER_RADIUS + 2.0;
const COLLISION_DIST_RED: f32 = PLAYER_RADIUS + RED_FLOWER_RADIUS + 2.0;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// A non-red flower; touching one triggers a respawn.
#[derive(Component)]
pub struct Flower;

/// The unique red flower; touching it triggers the win state.
#[derive(Component)]
pub struct RedFlower;

/// Marker for the "You found the red flower!" win overlay.
#[derive(Component)]
struct WinOverlay;

// ---------------------------------------------------------------------------
// Deterministic per-tile hash (mirrors the one in world.rs)
// ---------------------------------------------------------------------------

fn tile_hash(gx: i32, gy: i32) -> u32 {
    let x = (gx as u32).wrapping_mul(374_761_393);
    let y = (gy as u32).wrapping_mul(668_265_263);
    let h = x.wrapping_add(y);
    let h = h ^ (h >> 13);
    let h = h.wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

// ---------------------------------------------------------------------------
// Grid positions for non-red flowers
// ---------------------------------------------------------------------------

/// Isometric grid positions where non-red flowers are placed.
/// Spread across all six biomes.
const NON_RED_POSITIONS: &[(i32, i32)] = &[
    // Grasslands hub
    (3, 2), (-3, 4), (5, -2), (8, 6), (-6, -3),
    (10, 1), (-8, 7), (2, 10), (-4, -8), (7, -5),
    (0, 5), (4, -4), (-5, 2), (6, 8), (-9, 1),
    // Jungle island
    (41, 36), (44, 39), (40, 40), (45, 36), (43, 40),
    (38, 39), (42, 42), (46, 38),
    // Desert island
    (53, -38), (56, -41), (58, -37), (54, -44), (50, -40),
    // Tundra / snow island
    (-49, -45), (-46, -48), (-52, -43), (-44, -46), (-50, -40),
    // Volcanic island (dangerous – low flowers, bee must go fast)
    (-43, 46), (-46, 50),
    // Rocky outcrop
    (6, -55), (3, -53), (7, -52),
    // Bridge areas
    (10, 5), (20, 10), (30, 18), (-10, -10), (-20, -20),
];

/// Grid position of the unique red flower (on the far rocky outcrop).
const RED_FLOWER_POS: (i32, i32) = (5, -58);

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_flowers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Shared mesh handles
    let normal_mesh = meshes.add(Circle::new(FLOWER_RADIUS));
    let petal_mesh  = meshes.add(Circle::new(FLOWER_RADIUS * 0.5));
    let red_mesh    = meshes.add(Circle::new(RED_FLOWER_RADIUS));
    let red_petal   = meshes.add(Circle::new(RED_FLOWER_RADIUS * 0.5));

    // Petal colours (pink, white, lavender – none red)
    let pink_mat    = materials.add(Color::srgb(1.00, 0.55, 0.75));
    let white_mat   = materials.add(Color::srgb(0.95, 0.95, 1.00));
    let yellow_mat  = materials.add(Color::srgb(0.95, 0.85, 0.10));
    let center_mat  = materials.add(Color::srgb(0.95, 0.85, 0.10));

    // Red flower materials
    let red_mat         = materials.add(Color::srgb(0.95, 0.05, 0.05));
    let red_center_mat  = materials.add(Color::srgb(1.00, 0.85, 0.10));

    let stem_mat = materials.add(Color::srgb(0.20, 0.60, 0.12));
    let stem_mesh = meshes.add(Rectangle::new(2.5, 8.0));

    // --- Non-red flowers ---
    for &(gx, gy) in NON_RED_POSITIONS {
        let world_pos = grid_to_world(gx, gy);
        let z = grid_to_depth(gx, gy) + 1.0;

        // Small deterministic offset so flowers don't sit exactly on grid centres
        let h = tile_hash(gx, gy);
        let off_x = ((h % 24) as f32 - 12.0) * 1.8;
        let off_y = (((h >> 8) % 16) as f32 - 8.0) * 1.0;

        let fx = world_pos.x + off_x;
        let fy = world_pos.y + off_y;

        // Petal colour cycles through three options
        let petal_mat = match h % 3 {
            0 => pink_mat.clone(),
            1 => white_mat.clone(),
            _ => yellow_mat.clone(),
        };

        // Stem
        commands.spawn((
            Mesh2d(stem_mesh.clone()),
            MeshMaterial2d(stem_mat.clone()),
            Transform::from_xyz(fx, fy + 2.0, z - 0.001),
        ));

        // Flower body (petals + centre) as a parent entity with the `Flower` marker
        commands
            .spawn((
                Flower,
                Mesh2d(normal_mesh.clone()),
                MeshMaterial2d(petal_mat),
                Transform::from_xyz(fx, fy + 6.0, z),
            ))
            .with_children(|p| {
                // Yellow centre
                p.spawn((
                    Mesh2d(petal_mesh.clone()),
                    MeshMaterial2d(center_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                ));
            });
    }

    // --- Special red flower ---
    let (rgx, rgy) = RED_FLOWER_POS;
    let rpos = grid_to_world(rgx, rgy);
    let rz   = grid_to_depth(rgx, rgy) + 2.0;

    // Stem
    commands.spawn((
        Mesh2d(stem_mesh),
        MeshMaterial2d(stem_mat),
        Transform::from_xyz(rpos.x, rpos.y + 3.0, rz - 0.001),
    ));

    commands
        .spawn((
            RedFlower,
            Mesh2d(red_mesh),
            MeshMaterial2d(red_mat),
            Transform::from_xyz(rpos.x, rpos.y + 10.0, rz),
        ))
        .with_children(|p| {
            p.spawn((
                Mesh2d(red_petal),
                MeshMaterial2d(red_center_mat),
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
        });

    // Build the win-overlay UI (hidden by default via alpha=0)
    spawn_win_overlay(&mut commands);
}

// ---------------------------------------------------------------------------
// Win overlay
// ---------------------------------------------------------------------------

fn spawn_win_overlay(commands: &mut Commands) {
    commands
        .spawn((
            WinOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            // Render on top of the game; below the respawn fade (1000) so
            // if both are visible the win message still shows.
            GlobalZIndex(999),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("🌸 You found the red flower! 🌸"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.9)),
            ));
            parent.spawn((
                Text::new("Press Space to fly again"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.85)),
            ));
        });
}

// ---------------------------------------------------------------------------
// Collision systems
// ---------------------------------------------------------------------------

/// If the bee touches any non-red flower, trigger a respawn.
fn check_flower_collision(
    respawn:      Res<RespawnState>,
    player_query: Query<&Transform, With<Player>>,
    flower_query: Query<&Transform, With<Flower>>,
    mut respawn_state: ResMut<RespawnState>,
) {
    if respawn.mode != RespawnMode::Normal {
        return;
    }
    let Ok(player_tf) = player_query.get_single() else { return; };
    let bee_pos = player_tf.translation.truncate();

    for flower_tf in &flower_query {
        let flower_pos = flower_tf.translation.truncate();
        if bee_pos.distance_squared(flower_pos) < COLLISION_DIST_NORMAL * COLLISION_DIST_NORMAL {
            respawn_state.mode  = RespawnMode::FadingOut;
            respawn_state.alpha = 0.0;
            return;
        }
    }
}

/// If the bee touches the red flower, show the win overlay and reset the bee.
fn check_win_collision(
    keyboard:         Res<ButtonInput<KeyCode>>,
    red_flower_query: Query<&Transform, With<RedFlower>>,
    mut win_overlay:  Query<(&mut Visibility, &mut BackgroundColor), With<WinOverlay>>,
    mut player_query: Query<(&mut Transform, &mut BeeVelocity), With<Player>>,
    respawn:          Res<RespawnState>,
) {
    let Ok((mut overlay_vis, mut overlay_bg)) = win_overlay.get_single_mut() else {
        return;
    };

    // If already showing the win screen, wait for Space to restart
    if *overlay_vis == Visibility::Inherited || *overlay_vis == Visibility::Visible {
        if keyboard.just_pressed(KeyCode::Space) {
            *overlay_vis = Visibility::Hidden;
            overlay_bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            if let Ok((mut tf, mut vel)) = player_query.get_single_mut() {
                tf.translation.x = PLAYER_SPAWN.x;
                tf.translation.y = PLAYER_SPAWN.y;
                tf.rotation = Quat::IDENTITY;
                vel.0 = Vec2::ZERO;
            }
        }
        return;
    }

    if respawn.mode != RespawnMode::Normal {
        return;
    }

    let Ok(red_tf) = red_flower_query.get_single() else { return; };
    let flower_pos = red_tf.translation.truncate();

    if let Ok((bee_tf, mut vel)) = player_query.get_single_mut() {
        let bee_pos = bee_tf.translation.truncate();
        if bee_pos.distance_squared(flower_pos) < COLLISION_DIST_RED * COLLISION_DIST_RED {
            // Show the win overlay and stop the bee
            *overlay_vis = Visibility::Inherited;
            overlay_bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.65);
            vel.0 = Vec2::ZERO;
        }
    }
}
