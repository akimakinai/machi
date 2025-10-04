use avian3d::prelude::*;
use bevy::{color::palettes::tailwind::FUCHSIA_400, prelude::*};

use crate::{
    character::{
        CharacterController, MovementEvent, MovementEventKind,
        ai::{
            ActiveAiAction, AiActionResult, AiActionSystems, AiOf, BehaviorTreeRoot,
            CurrentAiActionResult, SequenceNode, TimeLimitNode,
        },
        player::Player,
    },
    physics::GameLayer,
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_sleep_action, chase_action_update).in_set(AiActionSystems::UpdateAction),
        )
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
        parent.spawn((
            AiOf(id),
            SequenceNode { repeat: true },
            BehaviorTreeRoot,
            ActiveAiAction,
            children![
                (
                    AiOf(id),
                    TimeLimitNode::from_seconds(10.0),
                    children![(AiOf(id), ChasePlayerAction)],
                ),
                (AiOf(id), SleepAction::from_seconds(5.0)),
            ],
        ));
    });
}

#[derive(Component)]
struct ChasePlayerAction;

fn chase_action_update(
    mut query: Query<(&AiOf, &ChasePlayerAction, &mut CurrentAiActionResult), With<ActiveAiAction>>,
    mut transforms: ParamSet<(Query<&Transform, With<Player>>, Query<&mut Transform>)>,
    mut commands: Commands,
) -> Result<()> {
    let Some(player_translation) = transforms
        .p0()
        .iter()
        .next()
        .map(|transform| transform.translation)
    else {
        return Err("No player found".into());
    };

    let mut enemy_transforms = transforms.p1();

    for (&AiOf(entity), _action, mut result) in &mut query {
        let Ok(mut enemy_transform) = enemy_transforms.get_mut(entity) else {
            continue;
        };
        let to_player = player_translation - enemy_transform.translation;
        let mut planar = Vec3::new(to_player.x, 0.0, to_player.z);

        if planar.length_squared() <= 0.5 {
            debug!("Enemy reached player");
            result.0 = Some(AiActionResult::Complete);
            continue;
        }

        planar = planar.normalize();
        enemy_transform.rotation = Quat::from_rotation_arc(-Vec3::Z, planar);

        commands.trigger(MovementEvent {
            entity,
            kind: MovementEventKind::Move(Vec2::Y),
        });

        result.0 = Some(AiActionResult::Continue);
    }

    Ok(())
}

#[derive(Component)]
struct SleepAction(Timer);

impl SleepAction {
    fn from_seconds(seconds: f32) -> Self {
        SleepAction(Timer::from_seconds(seconds, TimerMode::Once))
    }
}

fn update_sleep_action(
    mut query: Query<(&mut SleepAction, &mut CurrentAiActionResult), With<ActiveAiAction>>,
    time: Res<Time>,
) {
    let delta = time.delta();
    for (mut sleep, mut result) in &mut query {
        if sleep.0.tick(delta).just_finished() {
            sleep.0.reset();
            result.0 = Some(AiActionResult::Complete);
        } else {
            result.0 = Some(AiActionResult::Continue);
        }
    }
}
