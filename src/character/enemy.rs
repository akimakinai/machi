use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind::FUCHSIA_400,
    ecs::system::{
        StaticSystemParam,
        lifetimeless::{Read, SCommands, SQuery, Write},
    },
    prelude::*,
};

use crate::{
    character::{
        CharacterController, MovementEvent, MovementEventKind,
        ai::{ActiveAiAction, AiAction, AiActionPlugin, AiActionResult, AiOf},
        player::Player,
    },
    physics::GameLayer,
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AiActionPlugin::<ChasePlayerAction>::new())
            .add_systems(Startup, spawn_enemy);
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

    let mut enemy = commands.spawn((
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
        Transform::from_translation(Vec3::new(15.0, 20.0, 20.0)),
        CollisionLayers::new([GameLayer::Character], [GameLayer::Terrain]),
    ));
    let id = enemy.id();
    enemy.with_children(|parent| {
        parent.spawn((AiOf(id), ChasePlayerAction, ActiveAiAction));
    });
}

#[derive(Component)]
struct ChasePlayerAction;

impl AiAction for ChasePlayerAction {
    type Param = (
        SCommands,
        ParamSet<
            'static,
            'static,
            (
                SQuery<(Entity, Write<Transform>), With<Enemy>>,
                SQuery<Read<Transform>, With<Player>>,
            ),
        >,
    );

    fn update(
        &mut self,
        entity: Entity,
        _node_entity: Entity,
        params: &mut StaticSystemParam<Self::Param>,
    ) -> Result<AiActionResult> {
        let (commands, transforms) = &mut **params;
        let Some(player_translation) = transforms
            .p1()
            .iter()
            .next()
            .map(|transform| transform.translation)
        else {
            return Err("No player found".into());
        };

        let mut transforms = transforms.p0();
        let (entity, mut enemy_transform) = transforms.get_mut(entity)?;
        let to_player = player_translation - enemy_transform.translation;
        let mut planar = Vec3::new(to_player.x, 0.0, to_player.z);

        if planar.length_squared() <= 0.5 {
            debug!("Enemy reached player");
            return Ok(AiActionResult::Complete);
        }

        planar = planar.normalize();
        enemy_transform.rotation = Quat::from_rotation_arc(-Vec3::Z, planar);

        commands.trigger(MovementEvent {
            entity,
            kind: MovementEventKind::Move(Vec2::Y),
        });

        Ok(AiActionResult::Continue)
    }
}
