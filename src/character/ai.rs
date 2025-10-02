use std::marker::PhantomData;

use bevy::{
    ecs::{
        component::Mutable,
        system::{StaticSystemParam, SystemParam},
    },
    prelude::*,
};

use crate::pause::PausableSystems;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(FixedUpdate, AiActionUpdateSystems.in_set(PausableSystems));
    }
}

pub struct AiActionPlugin<T: AiAction>(PhantomData<T>);

impl<T: AiAction> AiActionPlugin<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: AiAction> Plugin for AiActionPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            ai_action_update::<T>.in_set(AiActionUpdateSystems),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct AiActionUpdateSystems;

// TODO: make this relation
#[derive(Component)]
pub struct AiOf(pub Entity);

fn ai_action_update<T: AiAction>(
    mut commands: Commands,
    mut query: Query<(&AiOf, Entity, &mut T), With<ActiveAiAction>>,
    mut params: StaticSystemParam<T::Param>,
) {
    for (&AiOf(entity), node_entity, mut action) in query.iter_mut() {
        match action.update(entity, node_entity, &mut params) {
            Ok(AiActionResult::Continue) => {}
            Ok(AiActionResult::Complete) => {
                commands.entity(node_entity).remove::<ActiveAiAction>();
            }
            Err(e) => {
                commands.entity(node_entity).remove::<ActiveAiAction>();
                error!(
                    "Error updating AI action for entity {:?}: {}",
                    node_entity, e
                );
            }
        }
    }
}

#[derive(Component)]
pub struct ActiveAiAction;

pub enum AiActionResult {
    Continue,
    Complete,
}

pub trait AiAction: Component<Mutability = Mutable> + 'static {
    type Param: SystemParam + 'static;

    fn update(
        &mut self,
        entity: Entity,
        node_entity: Entity,
        params: &mut StaticSystemParam<Self::Param>,
    ) -> Result<AiActionResult>;
}
