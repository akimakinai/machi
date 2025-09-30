use avian3d::prelude::*;
use bevy::{color::palettes::tailwind::FUCHSIA_400, prelude::*};

use crate::{
    character::{CharacterController, MovementEvent, MovementEventKind, player::Player},
    pause::PausableSystems,
    physics::GameLayer,
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemy)
            .add_systems(FixedUpdate, enemy_behavior.in_set(PausableSystems));
    }
}

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct Enemy;

fn spawn_enemy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shape = Sphere::new(0.5);
    let collider = shape.collider();

    commands.spawn((
        Name::new("Enemy"),
        Enemy,
        Mesh3d(meshes.add(shape.mesh().ico(32).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::from(FUCHSIA_400)))),
        Mass(2.0),
        Friction::new(0.5),
        collider,
        RigidBody::Dynamic,
        CharacterController {
            movement_acceleration: 50.0,
            ..default()
        },
        Transform::from_translation(Vec3::new(15.0, 20.0, 5.0)),
        CollisionLayers::new([GameLayer::Character], [GameLayer::Terrain]),
    ));
}

fn enemy_behavior(
    mut commands: Commands,
    mut transforms: ParamSet<(
        Query<(Entity, &mut Transform), With<Enemy>>,
        Query<&Transform, With<Player>>,
    )>,
) {
    let Some(player_translation) = transforms
        .p1()
        .iter()
        .next()
        .map(|transform| transform.translation)
    else {
        return;
    };

    for (entity, mut enemy_transform) in transforms.p0().iter_mut() {
        let to_player = player_translation - enemy_transform.translation;
        let mut planar = Vec3::new(to_player.x, 0.0, to_player.z);

        if planar.length_squared() <= f32::EPSILON {
            continue;
        }

        planar = planar.normalize();
        enemy_transform.rotation = Quat::from_rotation_arc(-Vec3::Z, planar);

        commands.trigger(MovementEvent {
            entity,
            kind: MovementEventKind::Move(Vec2::Y),
        });
    }
}
