use avian3d::prelude::*;
use bevy::{color::palettes::tailwind::FUCHSIA_400, prelude::*};

use crate::{
    character::{CharacterController, MovementBundle, Player},
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
        CharacterController,
        MovementBundle::default(),
        Transform::from_translation(Vec3::new(15.0, 20.0, 5.0)),
        CollisionLayers::new([GameLayer::Character], [GameLayer::Terrain]),
    ));
}

fn enemy_behavior(
    mut transforms: ParamSet<(
        Query<&mut Transform, With<Enemy>>,
        Query<&Transform, With<Player>>,
    )>,
    time: Res<Time>,
) {
    let Ok(player_transform) = transforms.p1().single().cloned() else {
        return;
    };

    for mut enemy_transform in &mut transforms.p0() {
        let direction = (player_transform.translation - enemy_transform.translation).normalize();
        enemy_transform.translation += direction * 2.0 * time.delta_secs();
    }
}
