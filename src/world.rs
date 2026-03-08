use bevy::{
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};

use crate::iso::{grid_to_depth, grid_to_world, TILE_HEIGHT, TILE_WIDTH};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_world);
    }
}

// ---------------------------------------------------------------------------
// Tile types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum TileType {
    Grass,
    Dirt,
    Rock,
}

impl TileType {
    fn color(self) -> Color {
        match self {
            TileType::Grass => Color::srgb(0.34, 0.62, 0.24),
            TileType::Dirt => Color::srgb(0.62, 0.45, 0.22),
            TileType::Rock => Color::srgb(0.55, 0.54, 0.53),
        }
    }
}

// ---------------------------------------------------------------------------
// Island generation
// ---------------------------------------------------------------------------

/// Fill a circular island patch into `tiles`, centred at `(cx, cy)`.
/// Tiles within `inner_radius_sq` use `inner_type`; the rest use `outer_type`.
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
                // Skip positions already occupied by a previous island
                if tiles.iter().any(|(x, y, _)| *x == gx && *y == gy) {
                    continue;
                }
                let tile_type = if dist_sq <= radius_sq / 3 {
                    inner_type
                } else {
                    outer_type
                };
                tiles.push((gx, gy, tile_type));
            }
        }
    }
}

fn generate_islands() -> Vec<(i32, i32, TileType)> {
    let mut tiles = Vec::new();

    // Main island – centred at origin
    island_circle(&mut tiles, 0, 0, 30, TileType::Grass, TileType::Dirt);

    // Northeast rocky outcrop
    island_circle(&mut tiles, 16, -11, 14, TileType::Rock, TileType::Dirt);

    // Southwest grass island
    island_circle(&mut tiles, -13, 9, 12, TileType::Grass, TileType::Dirt);

    // Southeast mixed island
    island_circle(&mut tiles, 11, 13, 18, TileType::Grass, TileType::Rock);

    // Northwest small rock island
    island_circle(&mut tiles, -15, -7, 8, TileType::Rock, TileType::Rock);

    tiles
}

// ---------------------------------------------------------------------------
// Mesh helpers
// ---------------------------------------------------------------------------

/// Build a diamond (rhombus) mesh the size of one isometric tile.
///
/// The shape has four vertices (top, right, bottom, left) arranged in
/// the classic 2:1 isometric diamond, and is made from two triangles.
fn create_tile_mesh() -> Mesh {
    let hw = TILE_WIDTH * 0.5;
    let hh = TILE_HEIGHT * 0.5;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    // Vertex positions: top, right, bottom, left
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0_f32, hh, 0.0],
            [hw, 0.0, 0.0],
            [0.0, -hh, 0.0],
            [-hw, 0.0, 0.0],
        ],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0_f32, 0.0, 1.0]; 4],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.5_f32, 0.0],
            [1.0, 0.5],
            [0.5, 1.0],
            [0.0, 0.5],
        ],
    );
    // Two triangles: (top, right, bottom) and (top, bottom, left)
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

    mesh
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tile_mesh = meshes.add(create_tile_mesh());

    for (gx, gy, tile_type) in generate_islands() {
        let pos = grid_to_world(gx, gy);
        let z = grid_to_depth(gx, gy);

        commands.spawn((
            Mesh2d(tile_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.color())),
            // Scale down slightly so there is a visible gap between tiles,
            // giving the isometric grid its characteristic look.
            Transform::from_xyz(pos.x, pos.y, z).with_scale(Vec3::new(0.93, 0.93, 1.0)),
        ));
    }
}
