use bevy::prelude::*;

use crate::pause::PausableSystems;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (
                AiActionSystems::PreUpdateAction,
                AiActionSystems::UpdateAction,
            )
                .chain()
                .in_set(PausableSystems),
        )
        .add_systems(
            FixedUpdate,
            pre_ai_action_update.in_set(AiActionSystems::PreUpdateAction),
        );
    }
}

// TODO: Control nodes in behavior tree (like Sequence) assigns `ActiveAiAction`
// in `PreUpdateAction` phase, action nodes run in `UpdateAction`

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AiActionSystems {
    PreUpdateAction,
    UpdateAction,
}

// TODO: make this relation
#[derive(Component)]
pub struct AiOf(pub Entity);

#[derive(Component)]
pub struct CurrentAiActionResult(pub Option<AiActionResult>);

fn pre_ai_action_update(
    mut commands: Commands,
    mut query: Query<(NameOrEntity, Entity, &mut CurrentAiActionResult), With<ActiveAiAction>>,
) {
    for (name, node_entity, mut result) in &mut query {
        match result.0.take() {
            Some(AiActionResult::Continue) => {}
            Some(AiActionResult::Complete) => {
                commands.entity(node_entity).remove::<ActiveAiAction>();
            }
            None => {
                error!("Active AI action ({name}) did not set result! Removing active action.");
                commands.entity(node_entity).remove::<ActiveAiAction>();
            }
        }
        // TODO: notify the parent behavior tree node
    }
}

#[derive(Component)]
pub struct ActiveAiAction;

pub enum AiActionResult {
    Continue,
    Complete,
}
