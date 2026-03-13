use std::collections::HashSet;

use bevy::prelude::*;

use crate::iso::{grid_to_world, world_to_grid, TILE_HEIGHT, TILE_WIDTH};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world);
    }
}

// ---------------------------------------------------------------------------
// LavaTiles resource
// ---------------------------------------------------------------------------

/// Set of every grid position that is a lava or volcanic tile.
/// The bee must cross these at high speed or it will be sent back to spawn.
#[derive(Resource, Default)]
pub struct LavaTiles(pub HashSet<(i32, i32)>);

impl LavaTiles {
    /// Returns `true` when `world_pos` is above at least one lava/volcanic tile.
    pub fn is_over_lava(&self, world_pos: Vec2) -> bool {
        let (gx, gy) = world_to_grid(world_pos);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if self.0.contains(&(gx + dx, gy + dy)) {
                    return true;
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Tile types
// ---------------------------------------------------------------------------

const TILE_THICKNESS: f32 = 9.0;
const BRIDGE_THICKNESS: f32 = 4.0;
const TILE_SCALE: f32 = 0.90;

#[derive(Clone, Copy, PartialEq)]
pub enum TileType {
    Grass,
    Dirt,
    Rock,
    Jungle,
    JungleFloor,
    Desert,
    DesertRock,
    Tundra,
    Snow,
    Volcanic,
    Lava,
    Bridge,
}

impl TileType {
    fn color(self) -> Color {
        match self {
            TileType::Grass => Color::srgb(0.18, 0.28, 0.10),
            TileType::Dirt => Color::srgb(0.20, 0.14, 0.07),
            TileType::Rock => Color::srgb(0.22, 0.22, 0.24),
            TileType::Jungle => Color::srgb(0.08, 0.20, 0.08),
            TileType::JungleFloor => Color::srgb(0.06, 0.12, 0.05),
            TileType::Desert => Color::srgb(0.25, 0.21, 0.12),
            TileType::DesertRock => Color::srgb(0.22, 0.15, 0.10),
            TileType::Tundra => Color::srgb(0.20, 0.23, 0.28),
            TileType::Snow => Color::srgb(0.32, 0.34, 0.40),
            TileType::Volcanic => Color::srgb(0.09, 0.06, 0.06),
            TileType::Lava => Color::srgb(0.25, 0.08, 0.02),
            TileType::Bridge => Color::srgb(0.16, 0.11, 0.06),
        }
    }

    fn roughness(self) -> f32 {
        match self {
            TileType::Snow | TileType::Tundra => 0.65,
            TileType::Lava => 0.45,
            TileType::Rock | TileType::Volcanic => 0.95,
            _ => 0.9,
        }
    }

    fn thickness(self) -> f32 {
        if matches!(self, TileType::Bridge) {
            BRIDGE_THICKNESS
        } else {
            TILE_THICKNESS
        }
    }
}

// ---------------------------------------------------------------------------
// Island / world generation
// ---------------------------------------------------------------------------

fn island_circle(
    tiles: &mut Vec<(i32, i32, TileType)>,
    cx: i32,
    cy: i32,
    radius_sq: i32,
    inner_type: TileType,
    outer_type: TileType,
) {
    let r = (radius_sq as f32).sqrt() as i32 + 1;
    for gx in (cx - r)..=(cx + r) {
        for gy in (cy - r)..=(cy + r) {
            let dx = gx - cx;
            let dy = gy - cy;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= radius_sq {
                if tiles.iter().any(|(x, y, _)| *x == gx && *y == gy) {
                    continue;
                }
                let tile_type = if dist_sq <= radius_sq / 2 {
                    inner_type
                } else {
                    outer_type
                };
                tiles.push((gx, gy, tile_type));
            }
        }
    }
}

fn add_bridge(tiles: &mut Vec<(i32, i32, TileType)>, from: (i32, i32), to: (i32, i32)) {
    let (x0, y0) = from;
    let (x1, y1) = to;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return;
    }

    let (ox, oy) = if dx.abs() >= dy.abs() { (0, 1) } else { (1, 0) };

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let bx = (x0 as f32 + dx as f32 * t).round() as i32;
        let by = (y0 as f32 + dy as f32 * t).round() as i32;

        for (sx, sy) in [(bx, by), (bx + ox, by + oy), (bx - ox, by - oy)] {
            if !tiles.iter().any(|(x, y, _)| *x == sx && *y == sy) {
                tiles.push((sx, sy, TileType::Bridge));
            }
        }
    }
}

fn generate_world() -> Vec<(i32, i32, TileType)> {
    let mut tiles = Vec::new();

    island_circle(&mut tiles, 0, 0, 280, TileType::Grass, TileType::Dirt);
    island_circle(&mut tiles, 42, 38, 220, TileType::JungleFloor, TileType::Jungle);
    island_circle(&mut tiles, 55, -40, 180, TileType::Desert, TileType::DesertRock);
    island_circle(&mut tiles, -48, -44, 240, TileType::Snow, TileType::Tundra);
    island_circle(&mut tiles, -44, 48, 160, TileType::Volcanic, TileType::Lava);
    island_circle(&mut tiles, 5, -55, 140, TileType::Rock, TileType::Dirt);

    add_bridge(&mut tiles, (0, 0), (42, 38));
    add_bridge(&mut tiles, (0, 0), (55, -40));
    add_bridge(&mut tiles, (0, 0), (-48, -44));
    add_bridge(&mut tiles, (0, 0), (-44, 48));
    add_bridge(&mut tiles, (0, 0), (5, -55));

    tiles
}

// ---------------------------------------------------------------------------
// Decorations (deterministic per-tile pseudo-random)
// ---------------------------------------------------------------------------

fn tile_hash(gx: i32, gy: i32) -> u32 {
    let x = (gx as u32).wrapping_mul(374_761_393);
    let y = (gy as u32).wrapping_mul(668_265_263);
    let h = x.wrapping_add(y);
    let h = h ^ (h >> 13);
    let h = h.wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn spawn_simple_mesh(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    translation: Vec3,
    scale: Vec3,
) {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(translation).with_scale(scale),
    ));
}

fn spawn_decorations(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    gx: i32,
    gy: i32,
    tile_type: TileType,
    ground_y: f32,
    world_pos: Vec2,
) {
    let h = tile_hash(gx, gy);
    let off_x = ((h % 20) as f32 - 10.0) * 0.7;
    let off_z = (((h >> 8) % 14) as f32 - 7.0) * 0.5;
    let base = Vec3::new(world_pos.x + off_x, ground_y, world_pos.y + off_z);

    match tile_type {
        TileType::Jungle | TileType::JungleFloor if h % 10 < 3 => {
            let trunk = meshes.add(Cuboid::new(2.2, 10.0, 2.2));
            let canopy = meshes.add(
                Sphere::new(6.0 + (h % 3) as f32)
                    .mesh()
                    .ico(3)
                    .expect("jungle canopy ico sphere should build"),
            );
            let trunk_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.10, 0.05),
                perceptual_roughness: 0.95,
                ..default()
            });
            let canopy_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.18, 0.05),
                perceptual_roughness: 0.95,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                trunk,
                trunk_mat,
                base + Vec3::new(0.0, 5.0, 0.0),
                Vec3::ONE,
            );
            spawn_simple_mesh(
                commands,
                canopy,
                canopy_mat,
                base + Vec3::new(0.0, 12.0, 0.0),
                Vec3::ONE,
            );
        }
        TileType::Desert | TileType::DesertRock if h % 10 < 2 => {
            let cactus = meshes.add(Cuboid::new(3.0, 12.0, 3.0));
            let arm = meshes.add(Cuboid::new(7.0, 2.0, 2.0));
            let cactus_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.10, 0.24, 0.11),
                perceptual_roughness: 0.95,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                cactus,
                cactus_mat.clone(),
                base + Vec3::new(0.0, 6.0, 0.0),
                Vec3::ONE,
            );
            spawn_simple_mesh(
                commands,
                arm,
                cactus_mat,
                base + Vec3::new(0.0, 8.0, 0.0),
                Vec3::ONE,
            );
        }
        TileType::Snow | TileType::Tundra if h % 8 < 2 => {
            let crystal = meshes.add(
                Sphere::new(2.5)
                    .mesh()
                    .ico(2)
                    .expect("tundra crystal ico sphere should build"),
            );
            let crystal_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.42, 0.48, 0.58),
                perceptual_roughness: 0.2,
                reflectance: 0.65,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                crystal,
                crystal_mat,
                base + Vec3::new(0.0, 2.5, 0.0),
                Vec3::new(0.8, 1.4, 0.8),
            );
        }
        TileType::Volcanic if h % 10 < 2 => {
            let rock = meshes.add(Cuboid::new(5.0, 1.4, 5.0));
            let rock_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.07, 0.04),
                perceptual_roughness: 0.95,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                rock,
                rock_mat,
                base + Vec3::new(0.0, 0.7, 0.0),
                Vec3::ONE,
            );
        }
        TileType::Grass if h % 12 < 1 => {
            let stone = meshes.add(
                Sphere::new(2.0)
                    .mesh()
                    .ico(2)
                    .expect("grass stone ico sphere should build"),
            );
            let stone_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.28, 0.18),
                perceptual_roughness: 0.95,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                stone,
                stone_mat,
                base + Vec3::new(0.0, 1.8, 0.0),
                Vec3::new(1.2, 0.5, 1.0),
            );
        }
        TileType::Rock if h % 10 < 1 => {
            let pebble = meshes.add(
                Sphere::new(2.2)
                    .mesh()
                    .ico(2)
                    .expect("rock pebble ico sphere should build"),
            );
            let pebble_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.28, 0.28, 0.30),
                perceptual_roughness: 0.95,
                ..default()
            });
            spawn_simple_mesh(
                commands,
                pebble,
                pebble_mat,
                base + Vec3::new(0.0, 1.6, 0.0),
                Vec3::new(1.1, 0.5, 0.9),
            );
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// World startup system
// ---------------------------------------------------------------------------

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let tiles = generate_world();

    let lava_set: HashSet<(i32, i32)> = tiles
        .iter()
        .filter(|(_, _, t)| matches!(t, TileType::Lava | TileType::Volcanic))
        .map(|(gx, gy, _)| (*gx, *gy))
        .collect();
    commands.insert_resource(LavaTiles(lava_set));

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.45, 0.48, 0.60),
            illuminance: 250.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-180.0, 260.0, 120.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    for &(gx, gy, tile_type) in &tiles {
        let pos = grid_to_world(gx, gy);
        let thickness = tile_type.thickness();
        let mesh = meshes.add(Cuboid::new(
            TILE_WIDTH * TILE_SCALE,
            thickness,
            TILE_HEIGHT * TILE_SCALE,
        ));
        let material = materials.add(StandardMaterial {
            base_color: tile_type.color(),
            perceptual_roughness: tile_type.roughness(),
            metallic: 0.02,
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(pos.x, thickness * 0.5, pos.y),
        ));

        spawn_decorations(
            &mut commands,
            &mut meshes,
            &mut materials,
            gx,
            gy,
            tile_type,
            thickness,
            pos,
        );
    }
}
