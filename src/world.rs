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

/// Visual height of the isometric tile side-face in pixels.
const TILE_SIDE_HEIGHT: f32 = 14.0;

/// Scale factor applied to every tile face so a small gap is visible between
/// adjacent tiles, giving the isometric grid its characteristic look.
const TILE_SCALE: f32 = 0.93;

#[derive(Clone, Copy, PartialEq)]
pub enum TileType {
    // Grassland biome
    Grass,
    Dirt,
    // Rocky outcrop
    Rock,
    // Jungle biome
    Jungle,
    JungleFloor,
    // Desert biome
    Desert,
    DesertRock,
    // Tundra / snow biome
    Tundra,
    Snow,
    // Volcanic biome
    Volcanic,
    Lava,
    // Bridge tiles connecting islands
    Bridge,
}

impl TileType {
    /// Top-face colour (what the player walks on).
    fn color(self) -> Color {
        match self {
            TileType::Grass       => Color::srgb(0.34, 0.62, 0.24),
            TileType::Dirt        => Color::srgb(0.62, 0.45, 0.22),
            TileType::Rock        => Color::srgb(0.55, 0.54, 0.53),
            TileType::Jungle      => Color::srgb(0.18, 0.52, 0.18),
            TileType::JungleFloor => Color::srgb(0.12, 0.35, 0.12),
            TileType::Desert      => Color::srgb(0.88, 0.78, 0.48),
            TileType::DesertRock  => Color::srgb(0.75, 0.58, 0.35),
            TileType::Tundra      => Color::srgb(0.78, 0.88, 0.94),
            TileType::Snow        => Color::srgb(0.94, 0.97, 1.00),
            TileType::Volcanic    => Color::srgb(0.22, 0.14, 0.12),
            TileType::Lava        => Color::srgb(0.92, 0.38, 0.05),
            TileType::Bridge      => Color::srgb(0.68, 0.52, 0.32),
        }
    }

    /// Left side-face colour (somewhat darker than the top face).
    fn side_color_left(self) -> Color {
        match self {
            TileType::Grass       => Color::srgb(0.22, 0.44, 0.15),
            TileType::Dirt        => Color::srgb(0.46, 0.32, 0.14),
            TileType::Rock        => Color::srgb(0.38, 0.38, 0.36),
            TileType::Jungle      => Color::srgb(0.10, 0.34, 0.10),
            TileType::JungleFloor => Color::srgb(0.07, 0.22, 0.07),
            TileType::Desert      => Color::srgb(0.70, 0.60, 0.34),
            TileType::DesertRock  => Color::srgb(0.58, 0.42, 0.24),
            TileType::Tundra      => Color::srgb(0.60, 0.72, 0.82),
            TileType::Snow        => Color::srgb(0.75, 0.82, 0.92),
            TileType::Volcanic    => Color::srgb(0.14, 0.08, 0.07),
            TileType::Lava        => Color::srgb(0.72, 0.25, 0.03),
            TileType::Bridge      => Color::srgb(0.50, 0.37, 0.20),
        }
    }

    /// Right side-face colour (darkest – simulates the shadow side).
    fn side_color_right(self) -> Color {
        match self {
            TileType::Grass       => Color::srgb(0.18, 0.36, 0.12),
            TileType::Dirt        => Color::srgb(0.38, 0.26, 0.11),
            TileType::Rock        => Color::srgb(0.30, 0.30, 0.28),
            TileType::Jungle      => Color::srgb(0.08, 0.26, 0.08),
            TileType::JungleFloor => Color::srgb(0.05, 0.16, 0.05),
            TileType::Desert      => Color::srgb(0.58, 0.50, 0.26),
            TileType::DesertRock  => Color::srgb(0.46, 0.34, 0.18),
            TileType::Tundra      => Color::srgb(0.50, 0.62, 0.72),
            TileType::Snow        => Color::srgb(0.65, 0.70, 0.82),
            TileType::Volcanic    => Color::srgb(0.10, 0.06, 0.05),
            TileType::Lava        => Color::srgb(0.55, 0.18, 0.02),
            TileType::Bridge      => Color::srgb(0.36, 0.26, 0.14),
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh builders
// ---------------------------------------------------------------------------

/// Diamond top-face mesh (2:1 isometric diamond).
fn create_top_mesh() -> Mesh {
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
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0_f32, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.5_f32, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]],
    );
    // Two triangles: (top, right, bottom) and (top, bottom, left)
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

/// Left side-face parallelogram: from the tile's left corner down to its
/// bottom vertex, then extruded downward by `TILE_SIDE_HEIGHT`.
fn create_left_side_mesh() -> Mesh {
    let hw = TILE_WIDTH * 0.5;
    let hh = TILE_HEIGHT * 0.5;
    let sh = TILE_SIDE_HEIGHT;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-hw, 0.0_f32, 0.0],   // top-left of top face
            [0.0, -hh, 0.0],        // bottom of top face
            [0.0, -hh - sh, 0.0],   // bottom of top face extruded down
            [-hw, -sh, 0.0],        // top-left extruded down
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0_f32, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0_f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

/// Right side-face parallelogram: from the tile's bottom vertex to its right
/// corner, extruded downward by `TILE_SIDE_HEIGHT`.
fn create_right_side_mesh() -> Mesh {
    let hw = TILE_WIDTH * 0.5;
    let hh = TILE_HEIGHT * 0.5;
    let sh = TILE_SIDE_HEIGHT;

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0_f32, -hh, 0.0],    // bottom of top face
            [hw, 0.0, 0.0],          // top-right of top face
            [hw, -sh, 0.0],          // top-right extruded down
            [0.0, -hh - sh, 0.0],    // bottom of top face extruded down
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0_f32, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0_f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

// ---------------------------------------------------------------------------
// Island / world generation
// ---------------------------------------------------------------------------

/// Fill a circular island into `tiles`, centred at `(cx, cy)`.
/// The inner half (by distance²) uses `inner_type`; the outer ring uses
/// `outer_type`.  Positions already occupied are skipped.
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

/// Trace a 3-tile-wide bridge path between two grid positions.
/// Only places `Bridge` tiles where no island tile already exists, so the
/// bridge naturally appears only in the gaps between islands.
fn add_bridge(tiles: &mut Vec<(i32, i32, TileType)>, from: (i32, i32), to: (i32, i32)) {
    let (x0, y0) = from;
    let (x1, y1) = to;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return;
    }

    // One-tile perpendicular offset to produce a 3-tile-wide deck
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

/// Build the full world: six biome islands connected by bridges.
fn generate_world() -> Vec<(i32, i32, TileType)> {
    let mut tiles = Vec::new();

    // 1. Central Grasslands – the starting hub
    island_circle(&mut tiles, 0, 0, 280, TileType::Grass, TileType::Dirt);

    // 2. Jungle – lush, dense canopy to the northeast
    island_circle(&mut tiles, 42, 38, 220, TileType::JungleFloor, TileType::Jungle);

    // 3. Desert – sun-scorched sands to the east
    island_circle(&mut tiles, 55, -40, 180, TileType::Desert, TileType::DesertRock);

    // 4. Tundra / Snow – frozen wastes to the northwest
    island_circle(&mut tiles, -48, -44, 240, TileType::Snow, TileType::Tundra);

    // 5. Volcanic – molten rock to the southwest
    island_circle(&mut tiles, -44, 48, 160, TileType::Volcanic, TileType::Lava);

    // 6. Rocky outcrop – a barren island to the south
    island_circle(&mut tiles, 5, -55, 140, TileType::Rock, TileType::Dirt);

    // Bridges (star topology, all connect back to the Grasslands hub)
    add_bridge(&mut tiles, (0, 0), (42, 38));    // Grasslands → Jungle
    add_bridge(&mut tiles, (0, 0), (55, -40));   // Grasslands → Desert
    add_bridge(&mut tiles, (0, 0), (-48, -44));  // Grasslands → Tundra
    add_bridge(&mut tiles, (0, 0), (-44, 48));   // Grasslands → Volcanic
    add_bridge(&mut tiles, (0, 0), (5, -55));    // Grasslands → Rocky

    tiles
}

// ---------------------------------------------------------------------------
// Decorations (deterministic per-tile pseudo-random)
// ---------------------------------------------------------------------------

/// Cheap deterministic hash for integer tile coordinates.
fn tile_hash(gx: i32, gy: i32) -> u32 {
    let x = (gx as u32).wrapping_mul(374_761_393);
    let y = (gy as u32).wrapping_mul(668_265_263);
    let h = x.wrapping_add(y);
    let h = h ^ (h >> 13);
    let h = h.wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Spawn small decoration meshes on top of a tile.
///
/// Each terrain type has its own decoration set; placement is deterministic
/// so the world looks the same every run.
fn spawn_decorations(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    gx: i32,
    gy: i32,
    tile_type: TileType,
    base_pos: Vec3,
) {
    let h = tile_hash(gx, gy);
    // Z just above the tile's top face
    let dz = base_pos.z + 0.002;
    // Small random offset within the tile
    let off_x = ((h % 20) as f32 - 10.0) * 0.7;
    let off_y = (((h >> 8) % 14) as f32 - 7.0) * 0.4;

    match tile_type {
        TileType::Jungle | TileType::JungleFloor => {
            if h % 10 < 3 {
                let canopy_r = 5.0 + (h % 4) as f32;
                // Trunk
                commands.spawn((
                    Mesh2d(meshes.add(Rectangle::new(3.0, 7.0))),
                    MeshMaterial2d(materials.add(Color::srgb(0.35, 0.22, 0.10))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y + 8.0, dz),
                ));
                // Canopy
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(canopy_r))),
                    MeshMaterial2d(materials.add(Color::srgb(0.08, 0.40, 0.08))),
                    Transform::from_xyz(
                        base_pos.x + off_x,
                        base_pos.y + off_y + 15.0,
                        dz + 0.001,
                    ),
                ));
            }
        }
        TileType::Desert | TileType::DesertRock => {
            if h % 10 < 2 {
                // Cactus body
                commands.spawn((
                    Mesh2d(meshes.add(Rectangle::new(4.0, 12.0))),
                    MeshMaterial2d(materials.add(Color::srgb(0.28, 0.55, 0.20))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y + 10.0, dz),
                ));
                // Cactus arms
                commands.spawn((
                    Mesh2d(meshes.add(Rectangle::new(9.0, 3.0))),
                    MeshMaterial2d(materials.add(Color::srgb(0.28, 0.55, 0.20))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y + 13.0, dz),
                ));
            }
        }
        TileType::Snow | TileType::Tundra => {
            if h % 8 < 2 {
                // Ice crystal
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(3.5))),
                    MeshMaterial2d(materials.add(Color::srgba(0.80, 0.92, 1.0, 0.85))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y + 5.0, dz),
                ));
            }
        }
        TileType::Volcanic => {
            if h % 10 < 2 {
                // Lava pool
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(4.5))),
                    MeshMaterial2d(materials.add(Color::srgb(0.95, 0.42, 0.05))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y, dz),
                ));
            }
        }
        TileType::Grass => {
            if h % 12 < 1 {
                let flower_color = match h % 3 {
                    0 => Color::srgb(1.0, 0.95, 0.2),
                    1 => Color::srgb(1.0, 0.5, 0.8),
                    _ => Color::srgb(0.9, 0.9, 1.0),
                };
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(3.0))),
                    MeshMaterial2d(materials.add(flower_color)),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y + 4.0, dz),
                ));
            }
        }
        TileType::Rock => {
            if h % 10 < 1 {
                // Pebble
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(3.0))),
                    MeshMaterial2d(materials.add(Color::srgb(0.45, 0.44, 0.43))),
                    Transform::from_xyz(base_pos.x + off_x, base_pos.y + off_y, dz),
                ));
            }
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
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Pre-build the three shared tile meshes
    let top_mesh   = meshes.add(create_top_mesh());
    let left_mesh  = meshes.add(create_left_side_mesh());
    let right_mesh = meshes.add(create_right_side_mesh());

    let scale = Vec3::new(TILE_SCALE, TILE_SCALE, 1.0);

    for (gx, gy, tile_type) in generate_world() {
        let pos = grid_to_world(gx, gy);
        let z   = grid_to_depth(gx, gy);
        let base = Vec3::new(pos.x, pos.y, z);

        // Top face
        commands.spawn((
            Mesh2d(top_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.color())),
            Transform::from_xyz(pos.x, pos.y, z).with_scale(scale),
        ));

        // Left side face (rendered just behind the top face in Z)
        commands.spawn((
            Mesh2d(left_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.side_color_left())),
            Transform::from_xyz(pos.x, pos.y, z - 0.003).with_scale(scale),
        ));

        // Right side face (darkest – simulates the shadow side)
        commands.spawn((
            Mesh2d(right_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.side_color_right())),
            Transform::from_xyz(pos.x, pos.y, z - 0.003).with_scale(scale),
        ));

        // Biome-specific decorations
        spawn_decorations(&mut commands, &mut meshes, &mut materials, gx, gy, tile_type, base);
    }
}
