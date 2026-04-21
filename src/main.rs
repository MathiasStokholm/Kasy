use bevy::prelude::*;

mod camera;
mod enemy;
mod flower;
mod iso;
mod player;
mod projectile;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Kasy – Bee & Flowers".to_string(),
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy".to_string()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.02)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.60, 0.62, 0.70),
            brightness: 550.0,
            ..default()
        })
        .add_plugins((
            world::WorldPlugin,
            camera::CameraPlugin,
            player::PlayerPlugin,
            flower::FlowerPlugin,
            enemy::EnemyPlugin,
            projectile::ProjectilePlugin,
        ))
        .run();
}
