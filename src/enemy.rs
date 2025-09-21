use avian3d::prelude::*;
use bevy::{color::palettes::tailwind::FUCHSIA_400, prelude::*};

use crate::character::CharacterController;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemy)
            .add_systems(Update, enemy_behavior);
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
            speed: 1.0,
            floating_height: 0.1,
            shape: Cylinder {
                radius: 1.0,
                half_height: 1.1,
            }
            .collider(),
        },
        Transform::from_translation(Vec3::new(15.0, 20.0, 5.0)),
    ));
}

fn enemy_behavior() {}
