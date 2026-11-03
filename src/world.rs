use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::render::{
    mesh::{Indices, PrimitiveTopology},
    render_asset::RenderAssetUsages,
};

use crate::enemy::SPAWN_POINTS;
use crate::flower::{NON_RED_POSITIONS, RED_FLOWER_POS};
use crate::iso::{grid_to_world, world_to_grid, TILE_SIZE};

/// Peak positions of mountains, placed on island interiors away from bridges.
const MOUNTAIN_PEAKS: &[(i32, i32)] = &[(50, 44), (66, -36), (-56, -40), (-4, -52)];
/// Extra stacked layers at a mountain peak.
const MOUNTAIN_PEAK_LAYERS: u32 = 3;
/// Grid distance per stacked layer step as you move away from a peak.
const MOUNTAIN_STEP: f32 = 1.6;

/// Compute how many layers each tile has.  Mountains are built from the
/// peaks outward.  Bridges, lava, and cells that hold flowers or enemy spawns
/// stay flat so gameplay placement is unaffected.
fn generate_heights(tiles: &[(i32, i32, TileType)]) -> HashMap<(i32, i32), u32> {
    let reserved: HashSet<(i32, i32)> = NON_RED_POSITIONS
        .iter()
        .copied()
        .chain(std::iter::once(RED_FLOWER_POS))
        .chain(SPAWN_POINTS.iter().copied())
        .collect();

    tiles
        .iter()
        .map(|&(gx, gy, tile_type)| {
            let mut extra = 0u32;
            let allowed = !matches!(
                tile_type,
                TileType::Bridge | TileType::Lava | TileType::Volcanic
            ) && !reserved.contains(&(gx, gy));
            if allowed {
                for &(px, py) in MOUNTAIN_PEAKS {
                    let dx = (gx - px) as f32;
                    let dy = (gy - py) as f32;
                    let steps = ((dx * dx + dy * dy).sqrt() / MOUNTAIN_STEP).floor();
                    let layers = MOUNTAIN_PEAK_LAYERS as f32 - steps;
                    if layers > 0.0 {
                        extra = extra.max(layers as u32);
                    }
                }
            }
            ((gx, gy), 1 + extra)
        })
        .collect()
}

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

/// Set of every grid position that has any tile.
/// Used by enemies to check they remain on the islands.
#[derive(Resource, Default)]
pub struct WorldTiles(pub HashSet<(i32, i32)>);

/// Number of stacked layers under each tile position.  Flat tiles have 1 layer;
/// mountains have more.  Missing tiles are absent from the map.
#[derive(Resource, Default)]
pub struct TerrainHeights(pub HashMap<(i32, i32), u32>);

impl TerrainHeights {
    /// Height of the top surface of the tile under `world_pos`, or 0 when there is no tile.
    pub fn ground_top(&self, world_pos: Vec2) -> f32 {
        let (gx, gy) = world_to_grid(world_pos);
        self.0
            .get(&(gx, gy))
            .map_or(0.0, |layers| *layers as f32 * TILE_THICKNESS)
    }

    /// Returns `true` when the tile under `world_pos` is stacked above flat ground.
    pub fn is_raised(&self, world_pos: Vec2) -> bool {
        self.ground_top(world_pos) > TILE_THICKNESS
    }
}

impl LavaTiles {
    /// Returns `true` when `world_pos` is above at least one lava/volcanic tile.
    pub fn is_over_lava(&self, world_pos: Vec2) -> bool {
        is_over_set(&self.0, world_pos)
    }
}

impl WorldTiles {
    /// Returns `true` when `world_pos` is above at least one tile of any type.
    pub fn is_over_tile(&self, world_pos: Vec2) -> bool {
        is_over_set(&self.0, world_pos)
    }
}

/// Checks whether `world_pos` maps to a grid cell (or any of its 8 neighbours)
/// that is present in `set`.  Used by both [`LavaTiles`] and [`WorldTiles`].
fn is_over_set(set: &HashSet<(i32, i32)>, world_pos: Vec2) -> bool {
    let (gx, gy) = world_to_grid(world_pos);
    for dx in -1..=1_i32 {
        for dy in -1..=1_i32 {
            if set.contains(&(gx + dx, gy + dy)) {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tile types
// ---------------------------------------------------------------------------

pub(crate) const TILE_THICKNESS: f32 = 9.0;

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
    fn sprite(
        self,
        grass: &Handle<Image>,
        sand: &Handle<Image>,
        road: &Handle<Image>,
    ) -> (Handle<Image>, Color) {
        let (image, tint) = match self {
            TileType::Grass => (grass, Color::WHITE),
            TileType::Dirt | TileType::Desert => (sand, Color::WHITE),
            TileType::Rock | TileType::Bridge => (road, Color::WHITE),
            TileType::Jungle => (grass, Color::srgb(0.74, 1.0, 0.64)),
            TileType::JungleFloor => (grass, Color::srgb(0.58, 0.84, 0.48)),
            TileType::DesertRock => (sand, Color::srgb(0.88, 0.72, 0.55)),
            TileType::Tundra | TileType::Snow => (road, Color::srgb(0.82, 0.92, 1.0)),
            TileType::Volcanic => (road, Color::srgb(0.72, 0.48, 0.38)),
            TileType::Lava => (road, Color::srgb(1.0, 0.38, 0.16)),
        };
        (image.clone(), tint)
    }
}

fn terrain_mesh() -> Mesh {
    let h = TILE_SIZE * 0.5;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[h, 0.0, -h], [h, 0.0, h], [-h, 0.0, h], [-h, 0.0, -h]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.5, 0.0],
            [1.0, 1.0 / 3.0],
            [0.5, 2.0 / 3.0],
            [0.0, 1.0 / 3.0],
        ],
    )
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]))
}

/// Side faces hanging below a tile's top surface, giving islands a cliff edge.
/// Interior faces between neighbouring tiles are hidden inside the island.
fn terrain_skirt_mesh() -> Mesh {
    let h = TILE_SIZE * 0.5;
    let corners = [
        Vec3::new(h, 0.0, -h),
        Vec3::new(h, 0.0, h),
        Vec3::new(-h, 0.0, h),
        Vec3::new(-h, 0.0, -h),
    ];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let normal = ((a + b) * 0.5 * Vec3::new(1.0, 0.0, 1.0)).normalize();
        let base = positions.len() as u32;
        positions.extend(
            [
                a,
                b,
                b - Vec3::Y * TILE_THICKNESS,
                a - Vec3::Y * TILE_THICKNESS,
            ]
            .map(|v| v.to_array()),
        );
        normals.extend([normal.to_array(); 4]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
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
    island_circle(
        &mut tiles,
        42,
        38,
        220,
        TileType::JungleFloor,
        TileType::Jungle,
    );
    island_circle(
        &mut tiles,
        55,
        -40,
        180,
        TileType::Desert,
        TileType::DesertRock,
    );
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
    asset_server: Res<AssetServer>,
) {
    let tiles = generate_world();
    let grass = asset_server.load("kenney/isometric-landscape/Grass.png");
    let sand = asset_server.load("kenney/isometric-landscape/Sand.png");
    let road = asset_server.load("kenney/isometric-landscape/Road.png");
    let tile_mesh = meshes.add(terrain_mesh());
    let skirt_mesh = meshes.add(terrain_skirt_mesh());
    let skirt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.30, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    });
    let stone_skirt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.46, 0.50),
        perceptual_roughness: 0.95,
        ..default()
    });

    let lava_set: HashSet<(i32, i32)> = tiles
        .iter()
        .filter(|(_, _, t)| matches!(t, TileType::Lava | TileType::Volcanic))
        .map(|(gx, gy, _)| (*gx, *gy))
        .collect();
    commands.insert_resource(LavaTiles(lava_set));

    let tile_set: HashSet<(i32, i32)> = tiles.iter().map(|(gx, gy, _)| (*gx, *gy)).collect();
    commands.insert_resource(WorldTiles(tile_set));

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.45, 0.48, 0.60),
            illuminance: 250.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-180.0, 260.0, 120.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let heights = generate_heights(&tiles);

    for &(gx, gy, tile_type) in &tiles {
        let pos = grid_to_world(gx, gy);
        let layers = heights.get(&(gx, gy)).copied().unwrap_or(1);
        let top = TILE_THICKNESS * layers as f32;
        let (image, tint) = tile_type.sprite(&grass, &sand, &road);
        let material = materials.add(StandardMaterial {
            base_color: tint,
            base_color_texture: Some(image),
            unlit: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Mesh3d(tile_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(pos.x, top, pos.y),
        ));
        for layer in 0..layers {
            let (mat, y) = if layer == 0 {
                (skirt_material.clone(), TILE_THICKNESS)
            } else {
                (
                    stone_skirt_material.clone(),
                    TILE_THICKNESS * (layer + 1) as f32,
                )
            };
            commands.spawn((
                Mesh3d(skirt_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(pos.x, y, pos.y),
            ));
        }

        spawn_decorations(
            &mut commands,
            &mut meshes,
            &mut materials,
            gx,
            gy,
            tile_type,
            top,
            pos,
        );
    }
    commands.insert_resource(TerrainHeights(heights));
}
