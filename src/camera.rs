use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    pbr::ShadowFilteringMethod,
    prelude::*,
};

use crate::player::Player;

pub struct CameraPlugin;

/// Fraction of the distance to the player that the camera closes each frame.
/// Lower values give a smoother (but lazier) follow; 1.0 snaps immediately.
const CAMERA_FOLLOW_SPEED: f32 = 0.08;
/// Offset chosen to keep the islands framed at a steep angle while still
/// reading as the old isometric playfield, just with real 3D depth.
const CAMERA_OFFSET: Vec3 = Vec3::new(-190.0, 180.0, 190.0);
const CAMERA_LOOK_HEIGHT: f32 = 14.0;

#[derive(Component)]
struct MainCamera;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_follow_player);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::ReinhardLuminance,
        Bloom::NATURAL,
        ShadowFilteringMethod::Gaussian,
        Transform::from_translation(CAMERA_OFFSET)
            .looking_at(Vec3::Y * CAMERA_LOOK_HEIGHT, Vec3::Y),
    ));
}

/// Smoothly slide the camera toward the player each frame.
fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    let look_target = player_transform.translation + Vec3::Y * CAMERA_LOOK_HEIGHT;
    let target = player_transform.translation + CAMERA_OFFSET;
    let new_pos = camera_transform
        .translation
        .lerp(target, CAMERA_FOLLOW_SPEED);

    *camera_transform = Transform::from_translation(new_pos).looking_at(look_target, Vec3::Y);
}
