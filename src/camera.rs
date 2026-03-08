use bevy::prelude::*;

use crate::player::Player;

pub struct CameraPlugin;

/// Fraction of the distance to the player that the camera closes each frame.
/// Lower values give a smoother (but lazier) follow; 1.0 snaps immediately.
const CAMERA_FOLLOW_SPEED: f32 = 0.08;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_follow_player);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Smoothly slide the camera toward the player each frame.
fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    let target = player_transform.translation.truncate();
    let current = camera_transform.translation.truncate();
    let new_pos = current.lerp(target, CAMERA_FOLLOW_SPEED);

    camera_transform.translation.x = new_pos.x;
    camera_transform.translation.y = new_pos.y;
}
