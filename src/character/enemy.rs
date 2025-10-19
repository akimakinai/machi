use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    character::{
        CharacterController, MovementEvent, MovementEventKind,
        ai::{
            ActiveNode, AiActionSystems, AiTarget, BehaviorTreeRoot, LeafNodeResult, SequenceNode,
            TimeLimitNode,
        },
        player::Player,
    },
    dev_util::{
        debug_annotation::{debug_annot_ui, target::AnnotTargetAabb},
        mesh_alpha::OverwriteAlpha,
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

#[derive(Component, Clone)]
#[require(Transform, Visibility)]
pub struct Enemy;

fn spawn_enemy(mut commands: Commands, asset_server: Res<AssetServer>) {
    let enemy_base = (
        Name::new("Enemy"),
        Enemy,
        Mass(2.0),
        Friction::new(0.5),
        RigidBody::Dynamic,
        CharacterController {
            movement_acceleration: 50.0,
            ..default()
        },
        CollisionLayers::new(
            [GameLayer::Character],
            [GameLayer::Terrain, GameLayer::Character],
        ),
        AnnotTargetAabb,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Enemy.glb"))),
        OverwriteAlpha(0.8),
    );

    for i in 0..3 {
        let position = Vec3::new(15.0 + i as f32 * 5.0, 20.0, 20.0 + i as f32 * 5.0);
        let mut enemy = commands.spawn((enemy_base.clone(), Transform::from_translation(position)));
        let id = enemy.id();
        enemy.with_children(|parent| {
            parent.spawn((
                SequenceNode { repeat: true },
                BehaviorTreeRoot::new(id),
                ActiveNode,
                children![
                    (
                        TimeLimitNode::from_seconds(10.0),
                        children![(ChasePlayerAction)],
                    ),
                    (SleepAction::from_seconds(5.0)),
                ],
            ));
        });

        commands.spawn(debug_annot_ui(id));
    }
}

#[derive(Component)]
struct ChasePlayerAction;

fn chase_action_update(
    ai_target: AiTarget,
    mut query: Query<(Entity, &ChasePlayerAction, &mut LeafNodeResult), With<ActiveNode>>,
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

    for (id, _action, mut result) in &mut query {
        let target = match ai_target.get_target(id) {
            Ok(target) => target,
            Err(e) => {
                error!("could not get target: {e}");
                continue;
            }
        };
        let Ok(mut enemy_transform) = enemy_transforms.get_mut(target) else {
            continue;
        };
        let to_player = player_translation - enemy_transform.translation;
        let mut planar = Vec3::new(to_player.x, 0.0, to_player.z);

        if planar.length_squared() <= 0.5 {
            debug!("Enemy reached player");
            result.set_complete();
            continue;
        }

        planar = planar.normalize();
        enemy_transform.rotation = Quat::from_rotation_arc(-Vec3::Z, planar);

        commands.trigger(MovementEvent {
            entity: target,
            kind: MovementEventKind::Move(Vec2::Y),
        });

        result.set_continue();
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
    mut query: Query<(&mut SleepAction, &mut LeafNodeResult), With<ActiveNode>>,
    time: Res<Time>,
) {
    let delta = time.delta();
    for (mut sleep, mut result) in &mut query {
        if sleep.0.tick(delta).just_finished() {
            sleep.0.reset();
            result.set_complete();
        } else {
            result.set_continue();
        }
    }
}
