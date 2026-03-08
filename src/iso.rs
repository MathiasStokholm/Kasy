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

/// Return a Z depth so that tiles closer to the camera (larger `gx + gy`)
/// are rendered on top of tiles that are further away.
pub fn grid_to_depth(gx: i32, gy: i32) -> f32 {
    (gx + gy) as f32 * 0.01
}
