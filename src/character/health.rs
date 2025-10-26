#![allow(dead_code)]

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

/// Add this component to an entity to give it health.
/// Observe [`DamageEvent`] and [`DeathEvent`] to respond to damage and death.
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    /// Creates a new `Health` with full health.
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct DamageEvent {
    entity: Entity,
    source: Option<Entity>,
    amount: f32,
}

impl DamageEvent {
    // No public constructor and only provide `Command` so that
    // `Health` is modified before events are sent.

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn source(&self) -> Option<Entity> {
        self.source
    }

    pub fn amount(&self) -> f32 {
        self.amount
    }
}

#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct DeathEvent {
    entity: Entity,
    source: Option<Entity>,
}

impl DeathEvent {
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn source(&self) -> Option<Entity> {
        self.source
    }
}

/// Modifies `Health` component of `target` entity and triggers [`DamageEvent`] or [`DeathEvent`].
pub fn deal_damage(target: Entity, source: Option<Entity>, amount: f32) -> impl Command {
    move |world: &mut World| {
        debug!("Dealing {} damage to {:?}", amount, target);

        let mut health = world.get_mut::<Health>(target).unwrap();

        if health.current <= 0.0 {
            // Already dead
            return;
        }

        health.current = (health.current - amount).max(0.0);

        if health.current == 0.0 {
            world.trigger(DeathEvent {
                entity: target,
                source,
            });
        } else {
            world.trigger(DamageEvent {
                entity: target,
                source,
                amount,
            });
        }
    }
}

/// Despawns the entity when it receives a `DeathEvent`.
/// Note that despawn is done in an observer.
#[derive(Component, Default)]
#[component(on_add = on_add_despawn_on_death)]
pub struct DespawnOnDeath;

fn on_add_despawn_on_death(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .entity(context.entity)
        .observe(despawn_on_death);
}

fn despawn_on_death(death: On<DeathEvent>, mut commands: Commands) {
    commands.entity(death.entity()).despawn();
}
