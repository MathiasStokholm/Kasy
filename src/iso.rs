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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_to_world_origin() {
        assert_eq!(grid_to_world(0, 0), Vec2::ZERO);
    }

    #[test]
    fn grid_to_world_unit_gx() {
        // gx=1, gy=0 → screen_x = TILE_WIDTH/2, screen_y = -TILE_HEIGHT/2
        assert_eq!(grid_to_world(1, 0), Vec2::new(TILE_WIDTH * 0.5, -TILE_HEIGHT * 0.5));
    }

    #[test]
    fn grid_to_world_unit_gy() {
        // gx=0, gy=1 → screen_x = -TILE_WIDTH/2, screen_y = -TILE_HEIGHT/2
        assert_eq!(grid_to_world(0, 1), Vec2::new(-TILE_WIDTH * 0.5, -TILE_HEIGHT * 0.5));
    }

    #[test]
    fn grid_to_world_negative_coords() {
        assert_eq!(grid_to_world(-1, -1), Vec2::new(0.0, TILE_HEIGHT));
    }

    #[test]
    fn world_to_grid_roundtrip() {
        for gx in -5..=5_i32 {
            for gy in -5..=5_i32 {
                let world = grid_to_world(gx, gy);
                assert_eq!(
                    world_to_grid(world),
                    (gx, gy),
                    "roundtrip failed for gx={gx}, gy={gy}"
                );
            }
        }
    }

    #[test]
    fn grid_to_depth_origin() {
        assert_eq!(grid_to_depth(0, 0), 0.0);
    }

    #[test]
    fn grid_to_depth_value() {
        // depth = (gx + gy) * 0.01
        assert!((grid_to_depth(3, 2) - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn grid_to_depth_ordering() {
        // Tiles closer to the camera (larger gx+gy) must have greater depth.
        assert!(grid_to_depth(2, 2) > grid_to_depth(0, 0));
        assert!(grid_to_depth(0, 0) > grid_to_depth(-1, -1));
    }
}
