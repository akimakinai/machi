use avian3d::prelude::*;
use bevy::{
    log::{DEFAULT_FILTER, LogPlugin},
    prelude::*,
};

use crate::{
    character::{CharacterController, Player},
    terrain::{
        chunk::{BlockId, Chunk, ChunkPlugin, ChunkUpdated},
        edit::EditPlugin,
        render::RenderPlugin,
    },
};

mod character;
mod flycam;
mod physics;
mod terrain;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: format!("{},{}=debug", DEFAULT_FILTER, env!("CARGO_PKG_NAME")),
            ..default()
        }))
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins(ChunkPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(EditPlugin)
        .add_plugins(character::CharacterPlugin)
        .add_systems(Startup, startup)
        .add_systems(Startup, (spawn_chunk, spawn_player))
        .run();
}

fn startup(mut commands: Commands, mut updated: MessageWriter<ChunkUpdated>) {
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

fn spawn_chunk(mut commands: Commands, mut updated: MessageWriter<ChunkUpdated>) {
    let mut ids = vec![];

    for cx in -1..=1 {
        for cz in -1..=1 {
            let mut chunk = Chunk::new(IVec2::new(cx, cz));

            for x in 0..16 {
                for z in 0..16 {
                    for y in 0..16 {
                        if cx == 0 && cz == 0 && (x == 4 || x == 2) && z == 4 && y == 15 {
                            continue;
                        }
                        chunk.set_block(
                            IVec3::new(x, y, z),
                            if y >= 14 { BlockId(1) } else { BlockId(2) },
                        );
                    }
                }
            }

            if cx == 1 && cz == 1 {
                chunk.set_block(IVec3::new(7, 20, 7), BlockId(65));
            }

            ids.push(commands.spawn(chunk).id());
        }
    }

    updated.write_batch(ids.into_iter().map(ChunkUpdated));
}

fn spawn_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let shape = Capsule3d {
        radius: 0.5,
        half_length: 1.0,
    };
    let collider = shape.collider();
    commands
        .spawn((
            Name::new("Player"),
            // Mesh3d(meshes.add(Mesh::from(shape))),
            // MeshMaterial3d(Handle::<StandardMaterial>::default()),
            Friction::new(1.0),
            CharacterController {
                speed: 10.0,
                floating_height: 5.0,
                shape: Cylinder {
                    radius: 1.0,
                    half_height: 1.1,
                }
                .collider(),
            },
            Transform::from_translation(Vec3::new(8.0, 25.0, 8.0)),
            Mass(1.0),
            collider,
            Player,
        ))
        .with_child((
            Camera3d::default(),
            // Transform::from_xyz(40.0, 30.0, 8.0).looking_at(Vec3::new(8.0, 16.0, 8.0), Vec3::Y),
            // FlyCam,
        ));
}
