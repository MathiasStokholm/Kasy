use bevy::prelude::*;

use crate::iso::{grid_to_world, world_to_plane};
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

const FLOWER_RADIUS: f32 = 6.0;
const RED_FLOWER_RADIUS: f32 = 9.0;
/// Height of the blossom center above the tile surface.
const FLOWER_HEIGHT: f32 = 10.0;
const STEM_HEIGHT: f32 = 9.0;
const STEM_MESH_HEIGHT: f32 = 8.0;
const FLOWER_LIGHT_HEIGHT_OFFSET: f32 = 1.5;
const FLOWER_GROUND_FILL_HEIGHT_OFFSET: f32 = 0.25;
const FLOWER_DOWNLIGHT_HEIGHT_OFFSET: f32 = 10.0;
const FLOWER_DOWNLIGHT_TARGET_HEIGHT_OFFSET: f32 = -1.0;
const FLOWER_DOWNLIGHT_INTENSITY_SCALE: f32 = 0.55;
const FLOWER_DOWNLIGHT_RANGE: f32 = 42.0;
const FLOWER_DOWNLIGHT_INNER_ANGLE: f32 = 0.55;
const FLOWER_DOWNLIGHT_OUTER_ANGLE: f32 = 1.05;
const FLOWER_GROUND_GLOW_HEIGHT_OFFSET: f32 = -0.94;
const FLOWER_GROUND_GLOW_RADIUS: f32 = 18.0;
const RED_FLOWER_GROUND_GLOW_RADIUS: f32 = 24.0;
const FLOWER_GROUND_FILL_INTENSITY_SCALE: f32 = 1.1;
const FLOWER_GROUND_FILL_RANGE_BONUS: f32 = 36.0;
const FLOWER_LIGHT_INTENSITY: f32 = 3_500_000.0;
const FLOWER_LIGHT_RANGE: f32 = 125.0;
const RED_FLOWER_LIGHT_INTENSITY: f32 = 5_500_000.0;
const RED_FLOWER_LIGHT_RANGE: f32 = 155.0;
const PLAYER_WIN_RESET_HEIGHT_OFFSET: f32 = 2.0;
const COLLISION_DIST_NORMAL: f32 = PLAYER_RADIUS + FLOWER_RADIUS + 2.0;
const COLLISION_DIST_RED: f32 = PLAYER_RADIUS + RED_FLOWER_RADIUS + 2.0;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct Flower;

#[derive(Component)]
pub struct RedFlower;

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

const NON_RED_POSITIONS: &[(i32, i32)] = &[
    (3, 2), (-3, 4), (5, -2), (8, 6), (-6, -3), (10, 1), (-8, 7), (2, 10), (-4, -8), (7, -5),
    (0, 5), (4, -4), (-5, 2), (6, 8), (-9, 1), (41, 36), (44, 39), (40, 40), (45, 36), (43, 40),
    (38, 39), (42, 42), (46, 38), (53, -38), (56, -41), (58, -37), (54, -44), (50, -40),
    (-49, -45), (-46, -48), (-52, -43), (-44, -46), (-50, -40), (-43, 46), (-46, 50), (6, -55),
    (3, -53), (7, -52), (10, 5), (20, 10), (30, 18), (-10, -10), (-20, -20),
];

const RED_FLOWER_POS: (i32, i32) = (5, -58);

fn emissive_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    emissive: LinearRgba,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive,
        perceptual_roughness: 0.35,
        reflectance: 0.4,
        ..default()
    })
}

fn spawn_flower_cluster(
    parent: &mut ChildBuilder,
    blossom_mesh: &Handle<Mesh>,
    center_mesh: &Handle<Mesh>,
    stem_mesh: &Handle<Mesh>,
    ground_glow_mesh: &Handle<Mesh>,
    petal_material: Handle<StandardMaterial>,
    center_material: Handle<StandardMaterial>,
    stem_material: Handle<StandardMaterial>,
    ground_glow_material: Handle<StandardMaterial>,
    light_color: Color,
    intensity: f32,
    range: f32,
    scale: f32,
    ground_glow_radius: f32,
) {
    parent.spawn((
        Mesh3d(stem_mesh.clone()),
        MeshMaterial3d(stem_material),
        Transform::from_xyz(0.0, -(STEM_HEIGHT * 0.5), 0.0)
            .with_scale(Vec3::new(1.0, STEM_HEIGHT / STEM_MESH_HEIGHT, 1.0)),
    ));

    for offset in [
        Vec3::new(2.2, 0.0, 0.0),
        Vec3::new(-2.2, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 2.2),
        Vec3::new(0.0, 0.0, -2.2),
    ] {
        parent.spawn((
            Mesh3d(blossom_mesh.clone()),
            MeshMaterial3d(petal_material.clone()),
            Transform::from_translation(offset).with_scale(Vec3::splat(scale)),
        ));
    }

    parent.spawn((
        Mesh3d(center_mesh.clone()),
        MeshMaterial3d(center_material),
        Transform::from_xyz(0.0, 0.8, 0.0).with_scale(Vec3::splat(scale)),
    ));

    parent.spawn((
        Mesh3d(ground_glow_mesh.clone()),
        MeshMaterial3d(ground_glow_material),
        Transform::from_xyz(0.0, FLOWER_GROUND_GLOW_HEIGHT_OFFSET, 0.0)
            .with_scale(Vec3::new(ground_glow_radius, 1.0, ground_glow_radius)),
    ));

    parent.spawn((
        PointLight {
            color: light_color,
            intensity,
            range,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, FLOWER_LIGHT_HEIGHT_OFFSET, 0.0),
    ));

    parent.spawn((
        PointLight {
            color: light_color,
            intensity: intensity * FLOWER_GROUND_FILL_INTENSITY_SCALE,
            range: range + FLOWER_GROUND_FILL_RANGE_BONUS,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, FLOWER_GROUND_FILL_HEIGHT_OFFSET, 0.0),
    ));

    parent.spawn((
        SpotLight {
            color: light_color,
            intensity: intensity * FLOWER_DOWNLIGHT_INTENSITY_SCALE,
            range: FLOWER_DOWNLIGHT_RANGE,
            inner_angle: FLOWER_DOWNLIGHT_INNER_ANGLE,
            outer_angle: FLOWER_DOWNLIGHT_OUTER_ANGLE,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, FLOWER_DOWNLIGHT_HEIGHT_OFFSET, 0.0).looking_at(
            Vec3::new(0.0, FLOWER_DOWNLIGHT_TARGET_HEIGHT_OFFSET, 0.0),
            Vec3::Z,
        ),
    ));
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn spawn_flowers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let blossom_mesh = meshes.add(
        Sphere::new(FLOWER_RADIUS * 0.42)
            .mesh()
            .ico(3)
            .expect("flower blossom ico sphere should build"),
    );
    let center_mesh = meshes.add(
        Sphere::new(FLOWER_RADIUS * 0.26)
            .mesh()
            .ico(3)
            .expect("flower center ico sphere should build"),
    );
    let stem_mesh = meshes.add(Cuboid::new(1.6, STEM_MESH_HEIGHT, 1.6));
    let ground_glow_mesh = meshes.add(Cylinder::new(1.0, 0.05));

    let pink_mat = emissive_material(
        &mut materials,
        Color::srgb(1.0, 0.55, 0.75),
        LinearRgba::rgb(8.0, 3.4, 5.5),
    );
    let white_mat = emissive_material(
        &mut materials,
        Color::srgb(0.95, 0.95, 1.0),
        LinearRgba::rgb(6.5, 6.5, 7.2),
    );
    let yellow_mat = emissive_material(
        &mut materials,
        Color::srgb(0.95, 0.85, 0.10),
        LinearRgba::rgb(8.5, 7.0, 1.8),
    );
    let center_mat = emissive_material(
        &mut materials,
        Color::srgb(1.0, 0.9, 0.18),
        LinearRgba::rgb(9.0, 7.5, 1.6),
    );
    let stem_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.28, 0.08),
        perceptual_roughness: 0.95,
        ..default()
    });
    let warm_ground_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.90, 0.62, 0.14),
        emissive: LinearRgba::rgb(0.35, 0.30, 0.18),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let red_mat = emissive_material(
        &mut materials,
        Color::srgb(0.95, 0.10, 0.10),
        LinearRgba::rgb(14.0, 2.2, 2.2),
    );
    let red_center_mat = emissive_material(
        &mut materials,
        Color::srgb(1.0, 0.88, 0.18),
        LinearRgba::rgb(10.0, 7.0, 1.5),
    );
    let red_ground_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.40, 0.24, 0.16),
        emissive: LinearRgba::rgb(0.40, 0.12, 0.08),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    for &(gx, gy) in NON_RED_POSITIONS {
        let world_pos = grid_to_world(gx, gy);
        let h = tile_hash(gx, gy);
        let off_x = ((h % 24) as f32 - 12.0) * 1.8;
        let off_z = (((h >> 8) % 16) as f32 - 8.0) * 1.0;
        let petal_mat = match h % 3 {
            0 => pink_mat.clone(),
            1 => white_mat.clone(),
            _ => yellow_mat.clone(),
        };

        commands
            .spawn((
                Flower,
                Transform::from_xyz(world_pos.x + off_x, FLOWER_HEIGHT, world_pos.y + off_z),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::VISIBLE,
                ViewVisibility::default(),
            ))
            .with_children(|parent| {
                spawn_flower_cluster(
                    parent,
                    &blossom_mesh,
                    &center_mesh,
                    &stem_mesh,
                    &ground_glow_mesh,
                    petal_mat,
                    center_mat.clone(),
                    stem_mat.clone(),
                    warm_ground_glow_mat.clone(),
                    Color::srgb(1.0, 0.85, 0.65),
                    FLOWER_LIGHT_INTENSITY,
                    FLOWER_LIGHT_RANGE,
                    1.0,
                    FLOWER_GROUND_GLOW_RADIUS,
                );
            });
    }

    let (rgx, rgy) = RED_FLOWER_POS;
    let rpos = grid_to_world(rgx, rgy);
    commands
        .spawn((
            RedFlower,
            Transform::from_xyz(rpos.x, FLOWER_HEIGHT + 1.0, rpos.y),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
            ViewVisibility::default(),
        ))
        .with_children(|parent| {
            spawn_flower_cluster(
                parent,
                &blossom_mesh,
                &center_mesh,
                &stem_mesh,
                &ground_glow_mesh,
                red_mat,
                red_center_mat,
                stem_mat,
                red_ground_glow_mat,
                Color::srgb(1.0, 0.22, 0.18),
                RED_FLOWER_LIGHT_INTENSITY,
                RED_FLOWER_LIGHT_RANGE,
                1.35,
                RED_FLOWER_GROUND_GLOW_RADIUS,
            );
        });

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

fn check_flower_collision(
    mut respawn_state: ResMut<RespawnState>,
    player_query: Query<&Transform, With<Player>>,
    flower_query: Query<&Transform, With<Flower>>,
) {
    if respawn_state.won || respawn_state.mode != RespawnMode::Normal {
        return;
    }
    let Ok(player_tf) = player_query.get_single() else {
        return;
    };
    let bee_pos = world_to_plane(player_tf.translation);

    for flower_tf in &flower_query {
        let flower_pos = world_to_plane(flower_tf.translation);
        if bee_pos.distance_squared(flower_pos) < COLLISION_DIST_NORMAL * COLLISION_DIST_NORMAL {
            respawn_state.mode = RespawnMode::FadingOut;
            respawn_state.alpha = 0.0;
            return;
        }
    }
}

fn check_win_collision(
    keyboard: Res<ButtonInput<KeyCode>>,
    red_flower_query: Query<&Transform, (With<RedFlower>, Without<Player>)>,
    mut win_overlay: Query<(&mut Visibility, &mut BackgroundColor), With<WinOverlay>>,
    mut player_query: Query<(&mut Transform, &mut BeeVelocity), (With<Player>, Without<RedFlower>)>,
    mut respawn: ResMut<RespawnState>,
) {
    let Ok((mut overlay_vis, mut overlay_bg)) = win_overlay.get_single_mut() else {
        return;
    };

    if *overlay_vis == Visibility::Inherited || *overlay_vis == Visibility::Visible {
        if keyboard.just_pressed(KeyCode::Space) {
            *overlay_vis = Visibility::Hidden;
            overlay_bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            respawn.won = false;
            if let Ok((mut tf, mut vel)) = player_query.get_single_mut() {
                tf.translation = Vec3::new(
                    PLAYER_SPAWN.x,
                    FLOWER_HEIGHT + PLAYER_WIN_RESET_HEIGHT_OFFSET,
                    PLAYER_SPAWN.y,
                );
                tf.rotation = Quat::IDENTITY;
                vel.0 = Vec2::ZERO;
            }
        }
        return;
    }

    if respawn.mode != RespawnMode::Normal {
        return;
    }

    let Ok(red_tf) = red_flower_query.get_single() else {
        return;
    };
    let flower_pos = world_to_plane(red_tf.translation);

    if let Ok((bee_tf, mut vel)) = player_query.get_single_mut() {
        let bee_pos = world_to_plane(bee_tf.translation);
        if bee_pos.distance_squared(flower_pos) < COLLISION_DIST_RED * COLLISION_DIST_RED {
            *overlay_vis = Visibility::Inherited;
            overlay_bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.65);
            vel.0 = Vec2::ZERO;
            respawn.won = true;
        }
    }
}
