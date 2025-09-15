use bevy::{
    log::{DEFAULT_FILTER, LogPlugin},
    prelude::*,
};

use crate::{
    camera_controller::{CameraController, CameraControllerPlugin},
    chunk::{Chunk, ChunkPlugin},
    render::RenderPlugin,
};

mod camera_controller;
mod chunk;
mod render;

fn main() {
    println!("{:?}", std::env::var("RUST_LOG"));

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ChunkPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(CameraControllerPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    let mut chunk = Chunk::new(IVec2::new(0, 0));

    for x in 0..16 {
        for z in 0..16 {
            for y in 0..16 {
                chunk.set_block(IVec3::new(x, y, z), 2);
            }
        }
    }

    commands.spawn(chunk);

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(40.0, 30.0, 8.0).looking_at(Vec3::new(8.0, 16.0, 8.0), Vec3::Y),
        CameraController::default(),
    ));

    commands.spawn((
        DirectionalLight { ..default() },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));
}
