use bevy::{
    image::ImageSampler,
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
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
// Pixel-art tile texture generation
// ---------------------------------------------------------------------------

/// Edge length (in pixels) of every tile's square texture atlas.
const TILE_TEX_SIZE: u32 = 32;

/// Per-pixel hash – drives all noise and variation inside a tile image.
fn pixel_hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(374_761_393);
    h = h.wrapping_add(y.wrapping_mul(668_265_263));
    h = h.wrapping_add(seed.wrapping_mul(2_654_435_769));
    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Clamp i32 to 0–255.
#[inline]
fn c8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Shortest distance from point (px, py) to line segment (x0,y0)–(x1,y1).
fn dist_to_seg(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        return ((px - x0) * (px - x0) + (py - y0) * (py - y0)).sqrt();
    }
    let t = (((px - x0) * dx + (py - y0) * dy) / len_sq).clamp(0.0, 1.0);
    let (qx, qy) = (x0 + t * dx - px, y0 + t * dy - py);
    (qx * qx + qy * qy).sqrt()
}

// ── Per-biome pixel functions ──────────────────────────────────────────────

fn px_grass(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 7);
    let h2 = pixel_hash(x, y, 13);
    // Vertical grass-blade streaks
    let blade: i32 = match x % 5 { 0 => -12, 1 => 14, 3 => -6, _ => 0 };
    let v = (h % 28) as i32 - 14;
    let r = c8(80  + blade + v);
    let g = c8(158 + blade * 2 + v);
    let b = c8(60  + blade + v);
    if      h2 % 55 == 0 { [190, 220,  70] }   // yellow-green dew
    else if h2 % 80 == 0 { [220, 160, 200] }   // pink flower
    else                  { [r, g, b] }
}

fn px_dirt(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 3);
    let h2 = pixel_hash(x, y, 9);
    let v = (h % 30) as i32 - 15;
    if h2 % 40 == 0 && (h2 >> 8) % 3 == 0 {
        [c8(90 + v), c8(65 + v), c8(35 + v)]   // dark pebble
    } else {
        [c8(155 + v), c8(112 + v), c8(58 + v)]
    }
}

fn px_rock(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 11);
    let h2 = pixel_hash(x, y, 17);
    // Angular facets via diagonal bands
    let facet: i32 = match (x + y) % 10 { d if d < 3 => 18, d if d > 7 => -14, _ => 0 };
    let v = (h % 20) as i32 - 10;
    if h2 % 70 == 0 { [220, 215, 210] }   // mineral sparkle
    else { [c8(138 + facet + v), c8(135 + facet + v), c8(130 + facet + v)] }
}

fn px_jungle(x: u32, y: u32) -> [u8; 3] {
    let h = pixel_hash(x, y, 5);
    // Leaf-blob circles, one per 8×8 cell
    let (cx, cy) = (((x / 8) * 8 + 4) as i32, ((y / 8) * 8 + 4) as i32);
    let d = ((x as i32 - cx).pow(2) + (y as i32 - cy).pow(2)) as f32;
    let leaf: i32 = if d < 4.0 { 22 } else if d < 10.0 { 8 } else { -6 };
    let v = (h % 20) as i32 - 10;
    [c8(38 + leaf + v), c8(115 + leaf + v), c8(35 + leaf + v)]
}

fn px_jungle_floor(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 19);
    let h2 = pixel_hash(x, y, 23);
    let v = (h % 18) as i32 - 9;
    if h2 % 35 == 0 { [c8(80 + v), c8(95 + v), c8(30 + v)] }   // fallen leaf
    else { [c8(28 + v), c8(82 + v), c8(25 + v)] }
}

fn px_desert(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 2);
    let h2 = pixel_hash(x, y, 8);
    // Diagonal dune shading
    let dune: i32 = match (x + y) % 18 { d if d < 4 => 22, d if d > 14 => -16, _ => 0 };
    let v = (h % 22) as i32 - 11;
    if h2 % 30 == 0 { [c8(160 + v), c8(130 + v), c8(80 + v)] }   // rock fragment
    else { [c8(210 + dune + v), c8(190 + dune + v), c8(120 + dune + v / 2)] }
}

fn px_desert_rock(x: u32, y: u32) -> [u8; 3] {
    let h = pixel_hash(x, y, 14);
    // Fractured-rock crack lines
    let crack: i32 = if (x * 2 + y) % 12 == 0 || (x * 2 + y) % 12 == 6 { -30 } else { 0 };
    let v = (h % 20) as i32 - 10;
    [c8(185 + crack + v), c8(142 + crack + v), c8(88 + crack + v)]
}

fn px_tundra(x: u32, y: u32) -> [u8; 3] {
    let h = pixel_hash(x, y, 6);
    // Hexagonal ice-crystal star pattern (approximated with cardinal + diagonal axes)
    let (cx, cy) = ((x % 12) as i32 - 6, (y % 12) as i32 - 6);
    let on_star = cx == 0 || cy == 0 || cx == cy || cx == -cy;
    let crystal: i32 = if on_star && cx * cx + cy * cy <= 16 { 25 } else { 0 };
    let v = (h % 18) as i32 - 9;
    [c8(178 + crystal + v), c8(200 + crystal + v), c8(218 + crystal + v)]
}

fn px_snow(x: u32, y: u32) -> [u8; 3] {
    let h  = pixel_hash(x, y, 4);
    let h2 = pixel_hash(x, y, 12);
    let v = (h % 14) as i32 - 7;
    if h2 % 45 == 0 { [255, 255, 255] }   // sparkle
    else { [c8(235 + v), c8(240 + v), c8(252 + v)] }
}

fn px_volcanic(x: u32, y: u32) -> [u8; 3] {
    let (fx, fy) = (x as f32, y as f32);
    // Glowing lava-crack line segments
    const SEGS: &[(f32, f32, f32, f32)] = &[
        ( 0.0, 14.0, 32.0, 20.0),
        (10.0,  0.0, 18.0, 32.0),
        (20.0,  6.0, 32.0, 26.0),
        ( 0.0, 26.0, 14.0, 10.0),
    ];
    let min_d = SEGS.iter()
        .map(|&(x0, y0, x1, y1)| dist_to_seg(fx, fy, x0, y0, x1, y1))
        .fold(f32::INFINITY, f32::min);
    let h = pixel_hash(x, y, 1);
    let v = (h % 10) as i32 - 5;
    if      min_d < 1.2 { [255, 210,  80] }                                        // core
    else if min_d < 2.5 { [230, 120,  20] }                                        // hot orange
    else if min_d < 4.5 { [c8(120 + v), c8(40 + v), c8(10 + v)] }                // glow
    else                { [c8( 32 + v), c8(18 + v), c8(14 + v)] }                 // dark rock
}

fn px_lava(x: u32, y: u32) -> [u8; 3] {
    let h = pixel_hash(x, y, 16);
    // Cooling-crust blobs centred every 8×8 cell
    let (cx, cy) = ((x % 8) as i32 - 4, (y % 8) as i32 - 4);
    let crust: i32 = match cx * cx + cy * cy { d if d <= 2 => -90, d if d <= 5 => -50, _ => 0 };
    let v = (h % 25) as i32 - 12;
    [c8(232 + crust + v), c8(80 + crust / 3 + v), c8(10 + crust / 8 + v)]
}

fn px_bridge(x: u32, y: u32) -> [u8; 3] {
    let h = pixel_hash(x, y, 20);
    let within = y % 6;
    // Dark plank separator
    if within == 0 { return [65, 42, 18]; }
    // Alternate plank tones
    let base: [i32; 3] = if (y / 6) % 2 == 0 { [172, 128, 72] } else { [158, 115, 62] };
    // Subtle wood grain + occasional knot
    let grain = (h % 16) as i32 - 8;
    let knot = (x % 10) as i32 - 5;
    let ky   = within as i32 - 3;
    let dk: i32 = if h % 80 == 0 && knot * knot + ky * ky * 2 < 6 { -40 } else { 0 };
    [c8(base[0] + grain + dk), c8(base[1] + grain / 2 + dk), c8(base[2] + grain / 3 + dk)]
}

/// Generate a `TILE_TEX_SIZE`×`TILE_TEX_SIZE` RGBA pixel-art `Image` for the
/// given `tile_type`.  Nearest-neighbour sampling keeps the pixels crisp.
fn generate_tile_image(tile_type: TileType) -> Image {
    let size = TILE_TEX_SIZE;
    let mut data = vec![255u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let [r, g, b] = match tile_type {
                TileType::Grass       => px_grass(x, y),
                TileType::Dirt        => px_dirt(x, y),
                TileType::Rock        => px_rock(x, y),
                TileType::Jungle      => px_jungle(x, y),
                TileType::JungleFloor => px_jungle_floor(x, y),
                TileType::Desert      => px_desert(x, y),
                TileType::DesertRock  => px_desert_rock(x, y),
                TileType::Tundra      => px_tundra(x, y),
                TileType::Snow        => px_snow(x, y),
                TileType::Volcanic    => px_volcanic(x, y),
                TileType::Lava        => px_lava(x, y),
                TileType::Bridge      => px_bridge(x, y),
            };
            let i = ((y * size + x) * 4) as usize;
            data[i]     = r;
            data[i + 1] = g;
            data[i + 2] = b;
            // alpha = 255 (already set by vec initialization)
        }
    }
    let mut image = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Crisp pixel-art look – no blurring between pixels
    image.sampler = ImageSampler::nearest();
    image
}

/// Map a `TileType` to a stable index in the texture table.
fn tile_type_idx(t: TileType) -> usize {
    match t {
        TileType::Grass       => 0,
        TileType::Dirt        => 1,
        TileType::Rock        => 2,
        TileType::Jungle      => 3,
        TileType::JungleFloor => 4,
        TileType::Desert      => 5,
        TileType::DesertRock  => 6,
        TileType::Tundra      => 7,
        TileType::Snow        => 8,
        TileType::Volcanic    => 9,
        TileType::Lava        => 10,
        TileType::Bridge      => 11,
    }
}

// ---------------------------------------------------------------------------
// World startup system
// ---------------------------------------------------------------------------

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Pre-build the three shared tile meshes
    let top_mesh   = meshes.add(create_top_mesh());
    let left_mesh  = meshes.add(create_left_side_mesh());
    let right_mesh = meshes.add(create_right_side_mesh());

    let scale = Vec3::new(TILE_SCALE, TILE_SCALE, 1.0);

    // Build one pixel-art texture per tile type (shared across all tiles of that type)
    const ALL_TYPES: [TileType; 12] = [
        TileType::Grass, TileType::Dirt, TileType::Rock,
        TileType::Jungle, TileType::JungleFloor,
        TileType::Desert, TileType::DesertRock,
        TileType::Tundra, TileType::Snow,
        TileType::Volcanic, TileType::Lava,
        TileType::Bridge,
    ];
    let tex_handles: Vec<Handle<Image>> = ALL_TYPES
        .iter()
        .map(|&t| images.add(generate_tile_image(t)))
        .collect();

    for (gx, gy, tile_type) in generate_world() {
        let pos  = grid_to_world(gx, gy);
        let z    = grid_to_depth(gx, gy);
        let base = Vec3::new(pos.x, pos.y, z);

        // Top face – pixel-art texture
        let tex = tex_handles[tile_type_idx(tile_type)].clone();
        commands.spawn((
            Mesh2d(top_mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial {
                color: Color::WHITE,
                texture: Some(tex),
                ..default()
            })),
            Transform::from_xyz(pos.x, pos.y, z).with_scale(scale),
        ));

        // Left side face – solid darker colour
        commands.spawn((
            Mesh2d(left_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.side_color_left())),
            Transform::from_xyz(pos.x, pos.y, z - 0.003).with_scale(scale),
        ));

        // Right side face – darkest (shadow side)
        commands.spawn((
            Mesh2d(right_mesh.clone()),
            MeshMaterial2d(materials.add(tile_type.side_color_right())),
            Transform::from_xyz(pos.x, pos.y, z - 0.003).with_scale(scale),
        ));

        // Biome-specific decorations
        spawn_decorations(&mut commands, &mut meshes, &mut materials, gx, gy, tile_type, base);
    }
}
