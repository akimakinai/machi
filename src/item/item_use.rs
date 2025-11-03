use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

use crate::item::ItemStack;

/// Marker component for items that can be used.
/// Observe [`ItemUse`] events on [`ItemKind`] entity to handle item usage.
#[derive(Component, Clone, Default)]
pub struct UsableItem;

#[derive(EntityEvent)]
pub struct ItemUse {
    #[event_target]
    item_id: Entity,
    user: Entity,
    item: Entity,
}

impl ItemUse {
    pub fn new(user: Entity, item: Entity) -> impl FnOnce(Entity) -> Self {
        move |item_id| Self {
            item_id,
            user,
            item,
        }
    }

    pub fn user(&self) -> Entity {
        self.user
    }
}

#[derive(Component, Clone, Default)]
#[require(UsableItem)]
#[component(on_add = consume_on_use_added)]
pub struct ConsumeOnUse;

fn consume_on_use_added(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .entity(context.entity)
        .observe(on_use_consume);
}

fn on_use_consume(
    on: On<ItemUse>,
    mut items: Query<&mut ItemStack>,
    mut commands: Commands,
) -> Result<()> {
    let mut item = items.get_mut(on.event().item)?;
    item.decrease_quantity(1)?;
    if item.quantity() == 0 {
        commands.entity(on.event().item).despawn();
    }
    Ok(())
}
