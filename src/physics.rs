use avian3d::prelude::*;
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

#[derive(PhysicsLayer, Debug, Default)]
pub enum GameLayer {
    #[default]
    Default,
    Terrain,
    Character,
    Object,
    Projectile,
}

/// Component that wakes up colliding entities when the entity is despawned.
#[derive(Component)]
#[require(CollidingEntities)]
#[component(on_remove = on_remove_wake_colliding_entities)]
pub struct WakeCollidingEntitiesOnDespawn;

fn on_remove_wake_colliding_entities(mut world: DeferredWorld, context: HookContext) {
    let (mut entities, mut commands) = world.entities_and_commands();

    if let Ok(entity) = entities.get_mut(context.entity)
        && let Some(cols) = entity.get::<CollidingEntities>()
    {
        for &col in cols.iter() {
            commands.queue(WakeBody(col));
        }
    }
}
