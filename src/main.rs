use bevy::prelude::*;

mod camera;
mod enemy;
mod flower;
mod iso;
mod player;
mod projectile;
mod world;

fn main() {
    let mut plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Kasy – Bee & Flowers".to_string(),
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy".to_string()),
            #[cfg(target_arch = "wasm32")]
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        plugins = plugins.set(AssetPlugin {
            file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
            ..default()
        });
    }

    App::new()
        .add_plugins(plugins)
        .insert_resource(ClearColor(Color::srgb(0.40, 0.72, 0.92)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.78, 0.86, 1.0),
            brightness: 900.0,
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
