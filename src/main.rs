use avian3d::prelude::*;
use bevy::{
    log::{DEFAULT_FILTER, LogPlugin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use crate::{
    character::{CharacterController, CharacterPlugin, Player},
    enemy::EnemyPlugin,
    pause::{Pause, PausePlugin},
    physics::GameLayer,
    terrain::{
        chunk::{BlockId, Chunk, ChunkPlugin, ChunkUpdated},
        edit::EditPlugin,
        render::RenderPlugin,
    },
};

mod character;
// mod flycam;
mod enemy;
mod pause;
mod physics;
mod terrain;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: format!("{},{}=debug", DEFAULT_FILTER, env!("CARGO_PKG_NAME")),
            ..default()
        }))
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins(PausePlugin)
        .add_plugins(ChunkPlugin)
        .add_plugins(RenderPlugin)
        .add_plugins(EditPlugin)
        .add_plugins(CharacterPlugin)
        .add_plugins(EnemyPlugin)
        .configure_sets(
            FixedPostUpdate,
            PhysicsSet::StepSimulation.run_if(in_state(Pause(false))),
        )
        .add_systems(Startup, startup)
        .add_systems(Startup, (spawn_chunk, spawn_player))
        .add_systems(Update, mouse_grabbing)
        .run();
}

fn startup(mut commands: Commands) {
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
                            if y >= 15 { BlockId(1) } else { BlockId(2) },
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

#[derive(Component)]
pub struct PlayerCamera;

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shape = Capsule3d {
        radius: 0.5,
        half_length: 1.0,
    };
    let collider = shape.collider();
    commands
        .spawn((
            Name::new("Player"),
            Mesh3d(meshes.add(Mesh::from(shape))),
            MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgba(1.0, 1.0, 1.0, 0.0)))),
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
            CollisionLayers::new([GameLayer::Character], [GameLayer::Terrain]),
            Player,
        ))
        .with_child((
            Camera3d::default(),
            Transform::from_scale(Vec3::splat(2.0)),
            PlayerCamera,
        ));
}

fn mouse_grabbing(
    mut cursor_opt: Query<(&mut CursorOptions, &Window), With<PrimaryWindow>>,
    paused: Res<State<Pause>>,
) -> Result<()> {
    let (mut cursor_opt, window) = cursor_opt.single_mut()?;

    let (grab_mode, visible) = if paused.0 || !window.focused {
        (CursorGrabMode::None, true)
    } else {
        (CursorGrabMode::Locked, false)
    };

    cursor_opt
        .reborrow()
        .map_unchanged(|o| &mut o.grab_mode)
        .set_if_neq(grab_mode);
    cursor_opt
        .map_unchanged(|o| &mut o.visible)
        .set_if_neq(visible);

    Ok(())
}
