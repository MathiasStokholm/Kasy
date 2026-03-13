use bevy::prelude::*;

mod camera;
mod flower;
mod iso;
mod player;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Kasy – Bee & Flowers".to_string(),
                // On wasm32 these fields tell Bevy which canvas element to use
                // and to resize it to fill the parent element.
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy".to_string()),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        // Sky-blue background – the bee flies high above the islands
        .insert_resource(ClearColor(Color::srgb(0.42, 0.65, 0.90)))
        .add_plugins((
            world::WorldPlugin,
            camera::CameraPlugin,
            player::PlayerPlugin,
            flower::FlowerPlugin,
        ))
        .run();
}
