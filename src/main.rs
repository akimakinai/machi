use bevy::{
    log::{DEFAULT_FILTER, LogPlugin},
    prelude::*,
};

use crate::{
    flycam::{FlyCam, FlyCamPlugin},
    terrain::{
        chunk::{BlockId, Chunk, ChunkPlugin, ChunkUpdated},
        edit::EditPlugin,
        render::RenderPlugin,
    },
};

mod flycam;
mod terrain;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: format!("{},{}=debug", DEFAULT_FILTER, env!("CARGO_PKG_NAME")),
            ..default()
        }))
        .add_plugins(ChunkPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(EditPlugin)
        .add_plugins(FlyCamPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    let mut ids = vec![];

    for cx in -1..=1 {
        for cz in -1..=1 {
            let mut chunk = Chunk::new(IVec2::new(cx, cz));

            for x in 0..16 {
                for z in 0..16 {
                    for y in 0..16 {
                        if cx == 0 && cz == 0 && x == 4 && z == 4 && y == 15 {
                            continue;
                        }
                        chunk.set_block(
                            IVec3::new(x, y, z),
                            if y >= 14 { BlockId(1) } else { BlockId(2) },
                        );
                    }
                }
            }

            ids.push(commands.spawn(chunk).id());
        }
    }

    commands.trigger(ChunkUpdated(ids));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(40.0, 30.0, 8.0).looking_at(Vec3::new(8.0, 16.0, 8.0), Vec3::Y),
        FlyCam,
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

    commands.insert_resource(AmbientLight {
        brightness: 200.0,
        ..default()
    });
}
