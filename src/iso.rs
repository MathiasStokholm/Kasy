use bevy::prelude::*;

/// Side length of a square grid cell in world units. The 2:1 isometric camera
/// projects a cell of this size to a 64×32 diamond on screen.
pub const TILE_SIZE: f32 = 45.25;

/// Convert an isometric grid coordinate `(gx, gy)` into 2-D world-space.
///
/// The grid is square in world space; the isometric camera does the projection.
pub fn grid_to_world(gx: i32, gy: i32) -> Vec2 {
    Vec2::new(gx as f32 * TILE_SIZE, gy as f32 * TILE_SIZE)
}

/// Extract the 2-D gameplay-plane position from a 3-D world translation.
pub fn world_to_plane(world: Vec3) -> Vec2 {
    Vec2::new(world.x, world.z)
}

/// Convert a 2D world-space position back to the nearest grid coordinate.
/// This is the inverse of [`grid_to_world`].
pub fn world_to_grid(world: Vec2) -> (i32, i32) {
    (
        (world.x / TILE_SIZE).round() as i32,
        (world.y / TILE_SIZE).round() as i32,
    )
}
