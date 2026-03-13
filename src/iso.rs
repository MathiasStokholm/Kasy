use bevy::prelude::*;

/// Width of an isometric tile (diamond) in pixels.
pub const TILE_WIDTH: f32 = 64.0;
/// Height of an isometric tile (diamond) in pixels.
pub const TILE_HEIGHT: f32 = 32.0;

/// Convert an isometric grid coordinate `(gx, gy)` into 2-D world-space.
///
/// The standard 2:1 isometric mapping is used:
/// ```text
///   screen_x = (gx - gy) * (TILE_WIDTH  / 2)
///   screen_y = (gx + gy) * (TILE_HEIGHT / 2)   (positive = up in Bevy)
/// ```
pub fn grid_to_world(gx: i32, gy: i32) -> Vec2 {
    Vec2::new(
        (gx - gy) as f32 * TILE_WIDTH * 0.5,
        -(gx + gy) as f32 * TILE_HEIGHT * 0.5,
    )
}

/// Convert a 2D world-space position back to the nearest isometric grid coordinate.
///
/// This is the inverse of [`grid_to_world`]:
/// ```text
///   gx = round((world_x / (TILE_WIDTH/2) - world_y / (TILE_HEIGHT/2)) / 2)
///   gy = round((-world_x / (TILE_WIDTH/2) - world_y / (TILE_HEIGHT/2)) / 2)
/// ```
pub fn world_to_grid(world: Vec2) -> (i32, i32) {
    let u = world.x / (TILE_WIDTH * 0.5);
    let v = -world.y / (TILE_HEIGHT * 0.5);
    let gx = ((u + v) * 0.5).round() as i32;
    let gy = ((v - u) * 0.5).round() as i32;
    (gx, gy)
}
